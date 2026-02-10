mod utility_files;
mod struct_files;
mod run_wasm;
mod server;

use crate::struct_files::wasm_list::*;
use crate::utility_files::wasm_loader::*;
use crate::utility_files::wasm_destroyer::*;
use crate::server::{get_stats, get_tasks, upload_task, start_task, stop_task, delete_task, AppState};
use actix_web::{web, App, HttpServer};
use actix_cors::Cors;
use actix_files::Files;
use std::sync::{Arc, Mutex};
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

    // CLI Loop
    loop {
        // We need to lock the list for CLI operations
        // Note: Some helper functions might take &mut WasmList, so we lock here.
        // Ideally helper functions would take Arc<Mutex<WasmList>> to minimize lock time,
        // but for now we lock for the duration of the menu action.
        
        let mainmenu: Vec<_> = vec![ 
            "Load .Wasm File",
            "Remove .Wasm File",
            "Runtime Metrics",
            "Start wasm",
            "Stop wasm",
            "Prioritize Wasm's",
            "Save Machine State",
            "Shutdown",
        ];

        let choice = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("WasmAsOS")
            .items(&mainmenu)
            .default(0)
            .interact()
            .unwrap();

        match choice {
            0 => {
                let mut list = shared_list.lock().unwrap();
                load_menu(&mut list);
            }, 
            1 => {
                let mut list = shared_list.lock().unwrap();
                remove_wasm(&mut list, 0);
            },
            2 => println!("Display Runtime Metrics"), 
            3 => {
                let mut list = shared_list.lock().unwrap();
                start_wasm(&mut list);
            },
            4 => {
                let mut list = shared_list.lock().unwrap();
                halt_wasm(&mut list);
            },
            5 => println!("Display Menu for prioritizing resources"), 
            6 => println!("Store the current state to a file"), 
            7 => {
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
