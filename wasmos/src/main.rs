//Dependacies
use dialoguer::{Select, theme::ColorfulTheme};

//Modules
mod utility_files;
mod struct_files;
use crate::struct_files::wasm_list::*;

fn main() {
    let mut wasmos_list = WasmList::new_list(); //List for storing wasm stuctures
    
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
            0 => utility_files::wasm_loader::load_menu(&mut wasmos_list), //Load Wasm File/s into list and txt file
            1 => utility_files::wasm_destroyer::remove_wasm(&mut wasmos_list),  //Remove wasm file from list and txt file
            2 => println!("Display Runtime Metrics"), //Display runtime metrics to the user
            3 => println!("Start a Wasm file"), //Load a menu for starting a wasm file.
            4 => println!("Halt Wasm file"),
            5 => println!("Diplay Menu for prioritizing recources"), //Scheduler
            6 => println!("Store the current state to a file"), //Optional but would be cool
            7 => break,
            _ => unreachable!(),
        }
        //clearscreen::clear().expect("ERROR Clearing Screen");
    }
}