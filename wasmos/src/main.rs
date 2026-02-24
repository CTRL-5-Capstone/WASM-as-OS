mod utility_files;
mod struct_files;
mod run_wasm;
mod server;

use crate::struct_files::wasm_list::*;
use crate::utility_files::wasm_loader::*;
use crate::utility_files::wasm_destroyer::*;
use crate::server::{get_stats, get_tasks, upload_task, start_task, stop_task, delete_task, AppState};
use crate::run_wasm::build_runtime::Runtime;
use actix_web::mime::Name;
use actix_web::{web, App, HttpServer};
use actix_cors::Cors;
use actix_files::Files;
use tokio::runtime;
use std::string;
use std::sync::mpsc::TryRecvError;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time;
use std::thread::{sleep, spawn};
use crate::run_wasm::wasm_control::*;
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut wasmos_list = WasmList::new_list();
    load_file(&mut wasmos_list);
    
    // Wrap in Arc<Mutex> for sharing between threads
    let shared_list = Arc::new(Mutex::new(wasmos_list));
    
    // Clone for the server
    let server_list = shared_list.clone();
    
    // Spawn the server in a background task
    tokio::spawn(async move {
        println!("Starting WASM-OS Server at http://localhost:8080");
        
        let app_state = web::Data::new(AppState {
            wasm_list: server_list,
        });

        let server = HttpServer::new(move || {
            let cors = Cors::default()
                .allow_any_origin()
                .allow_any_method()
                .allow_any_header();

            App::new()
                .wrap(cors)
                .app_data(app_state.clone())
                .app_data(web::JsonConfig::default().limit(52428800)) // 50MB limit
                .service(get_stats)
                .service(get_tasks)
                .service(upload_task)
                .service(start_task)
                .service(stop_task)
                .service(delete_task)
                .service(Files::new("/", "../web").index_file("index.html"))
        })
        .bind(("127.0.0.1", 8080))
        .expect("Failed to bind server");
        
        if let Err(e) = server.run().await {
            eprintln!("Server error: {}", e);
        }
    });

    // Give the server a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    let (to_thread, msgfrom_main): (Sender<Messages>, Receiver<Messages>) = channel();
    let (msgto_main, from_thread): (Sender<Messages>, Receiver<Messages>) = channel();
    let _lop_thread = spawn(||{runtime_loop(msgto_main, msgfrom_main);});
    let mut choice = 0;
    // CLI Loop
    loop {
        // We need to lock the list for CLI operations
        // Note: Some helper functions might take &mut WasmList, so we lock here.
        // Ideally helper functions would take Arc<Mutex<WasmList>> to minimize lock time,
        // but for now we lock for the duration of the menu action.
        let wasm_tup = shared_list.lock().unwrap().list_runningvec();
        let running_list = wasm_tup.1;
        let wasm_vec = wasm_tup.0;

        loop
        {
            match from_thread.try_recv()
            {
                Ok(mesg) => {
                    match mesg
                    {
                        Messages::Stop(wasm_name) =>
                        {
                            if let Ok(i) = running_list.binary_search(&wasm_name)
                            {
                                wasm_vec[i].lock().unwrap().wasm_file.run_false();
                            }
                        }
                        _ => panic!("Impossible Message from Thread"),
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => panic!("Critical Error Thread Unresponsive"),
            }
            sleep(time::Duration::from_millis(100));
        }  
        let mainmenu: Vec<_> = vec![ 
            "Load .Wasm File",
            "Remove .Wasm File",
            "Runtime Metrics",
            "Start Wasm",
            "Unpause Wasm",
            "Pause Wasm",
            "Stop Wasm",
            "Edit Wasms",
            "Prioritize Wasms",
            "Save Machine State",
            "Shutdown",
        ];

        choice = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("WasmAsOS")
            .items(&mainmenu)
            .default(choice)
            .interact()
            .unwrap();

        match choice {
            0 => {
                let mut list = shared_list.lock().unwrap();
                load_menu(&mut list);
            }, 
            1 => {
                let mut list = shared_list.lock().unwrap();
                remove_wasm(&mut list, 0, to_thread.clone());
            },
            2 => println!("Display Runtime Metrics"), 
            3 => {
                let mut list = shared_list.lock().unwrap();
                start_wasm(&mut list, to_thread.clone());
            },
            4 => pause_wasm(&mut shared_list.lock().unwrap(), to_thread.clone()),
            5 => unpause_wasm(&mut shared_list.lock().unwrap(), to_thread.clone()),
            6 => {
                let mut list = shared_list.lock().unwrap();
                halt_wasm(&mut list, to_thread.clone());
            },
            7 => edit_runtimes(&mut shared_list.lock().unwrap(), to_thread.clone()),
            8 => println!("Display Menu for prioritizing resources"), 
            9 => println!("Store the current state to a file"), 
            10 => {
                println!("Shutting down...");
                break;
            },
            _ => unreachable!(),
        }
    }
    
    // Cleanup
    let mut list = shared_list.lock().unwrap();
    cleanup_wasms(&mut list);

    Ok(())
}
