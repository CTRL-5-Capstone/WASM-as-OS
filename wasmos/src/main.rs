use dialoguer::{Select, theme::ColorfulTheme};
fn main() {
    let menu: Vec<_> = vec![
        "Load .Wasm File",
        "Runtime Metrics",
        "Start Wasm",
        "Halt Wasm",
        "Prioritize Wasm's",
        "Save Machine State",
        "Shutdown",
    ];

    loop {
        let choice = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("WasmAsOS")
            .items(&menu)
            .default(0)
            .interact()
            .unwrap();

        match choice {
            0 => println!("Load Wasm Code"),
            1 => println!("Display Runtime Metrics"),
            2 => println!("Start a Wasm file"),
            3 => println!("Halt Wasm file"),
            4 => println!("Diplay Menu for prioritizing recources"),
            5 => println!("Store the current state to a file"),
            6 => {
                println!("End Loop");
                break;
            }
            _ => unreachable!(),
        }
        println!(); // Add a newline for spacing
    }
}