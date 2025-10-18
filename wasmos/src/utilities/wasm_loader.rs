//Dependacies
use dialoguer::{Select, theme::ColorfulTheme};
use std::io::{self, Write};
use std::path::Path;

fn detect_wasm()
{
    println!("Detect Wasm files");
    
}
fn path_wasm()
{
    print!("Path: ");
    io::stdout().flush().expect("Failure to Flush  wasm_loader");

    let mut the_path = String::new();

    io::stdin()
        .read_line(&mut the_path)
        .expect("Path Input ERROR");
    let path_checker = Path::new(&the_path);
    if path_checker.exists() == false
    {
        println!("File Not Found");
    }
    else 
    {
        println!("Store Path and Filename")    
    }
    let path_men: Vec<_> = vec![
        
        "Add wasm File",
        "Return to Main Menu",
    ];

    let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Add File")
        .items(&path_men)
        .default(0)
        .interact()
        .unwrap();

    match choice {
        0 => path_wasm(),
        1 => return,
            _ => unreachable!(),
    }
}
pub fn load_menu()
{
    let loader_men: Vec<_> = vec![
        "Detect .wasm Files",
        "Enter Path",
        "Return to Main Menu",
    ];

    let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("File Menu")
        .items(&loader_men)
        .default(0)
        .interact()
        .unwrap();

    match choice {
        0 => detect_wasm(),
        1 => path_wasm(),
        2 => return,
            _ => unreachable!(),
    }
}