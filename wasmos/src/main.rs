//Dependacies
use dialoguer::{Select, theme::ColorfulTheme};

//Modules
mod utility_files;
mod struct_files;
mod run_wasm;
use crate::struct_files::wasm_list::*;
use crate::run_wasm::wasm_control::*;
use crate::utility_files::wasm_loader::*;
use crate::utility_files::wasm_destroyer::*;

fn main() {
    let mut wasmos_list = WasmList::new_list(); //List for storing wasm stuctures
    load_file(&mut wasmos_list);
    //Menu Subject to change
    //Need start, prioritize/schedule, runtime metrics for sure
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

    loop {
        let choice = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("WasmAsOS")
            .items(&mainmenu)
            .default(0)
            .interact()
            .unwrap();

        match choice {
            0 => load_menu(&mut wasmos_list), //Load Wasm File/s into list and txt file
            1 => remove_wasm(&mut wasmos_list, 0),  //Remove wasm file from list and txt file
            2 => println!("Display Runtime Metrics"), //Display runtime metrics to the user
            3 => start_wasm(&mut wasmos_list), //Load a menu for starting a wasm file.
            4 => halt_wasm(&mut wasmos_list),
            5 => println!("Diplay Menu for prioritizing recources"), //Scheduler
            6 => println!("Store the current state to a file"), //Optional but would be cool
            7 => break,
            _ => unreachable!(),
        }
        //clearscreen::clear().expect("ERROR Clearing Screen");
    }
    cleanup_wasms(&mut wasmos_list);
}