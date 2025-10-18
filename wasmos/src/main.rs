//Dependacies
use dialoguer::{Select, theme::ColorfulTheme};
use clearscreen;
//Modules
mod utilities;
fn main() {
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
            0 => utilities::wasm_loader::load_menu(),
            1 => println!("Delete.wasm Menu"),
            2 => println!("Display Runtime Metrics"),
            3 => println!("Start a Wasm file"),
            4 => println!("Halt Wasm file"),
            5 => println!("Diplay Menu for prioritizing recources"),
            6 => println!("Store the current state to a file"),
            7 => break,
            _ => unreachable!(),
        }
        //clearscreen::clear().expect("ERROR Clearing Screen");
    }
}