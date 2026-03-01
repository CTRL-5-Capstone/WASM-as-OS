//Dependacies
use dialoguer::{Select, theme::ColorfulTheme};
use std::io::{self, Write};
use std::path::Path;
use crate::struct_files::wasm_list::*;
use crate::struct_files::wasm_struct::WasmFile;
use std::fs;

pub fn load_file(wasm_list: &mut WasmList) //Loads wasm files from wasm_list.csv
{
    let mut itter = 0;
    let mut name = String::new();
    let from_file: String = fs::read_to_string("src/wasm_files/wasm_list.csv").expect("ERROR: Path to wasm_list.csv not found").trim().to_string();
    if !from_file.is_empty()
    {
        let wasm_vec: Vec<String> = from_file.split([',', '\n']).map(|to_string| to_string.trim().trim_matches([',', '\n']).to_string()).collect();
        for string in wasm_vec
        {
            if itter % 2 == 0
            {
                name = string;
            }
            else 
            {
                let path_to = string;
                let path_checker = Path::new(&path_to);
                if path_checker.exists()
                {
                    wasm_list.append_list(WasmFile::new_wasm(name.clone(), path_to));
                }    
            }
            itter += 1
        }
    }
}
    fn detect_wasm() //Implement a .wasm file detection method 
{
    println!("Detect Wasm files");
    
}
fn path_wasm(wasm_list: &mut WasmList) //Method for adding wasm files
{ 
    print!("Path: ");
    io::stdout().flush().expect("Failure to Flush wasm_loader");

    let mut the_path = String::new();
    
    //Get Wasm file path
    io::stdin().read_line(&mut the_path).expect("Path Input ERROR");
    
    let trimmed_path = the_path.trim().trim_matches('"'); //Remove parenthesis from path and trim whitespaces
    let path_checker = Path::new(&trimmed_path); //Create Path variable to check if file exisits
    if !path_checker.exists() //Case for file not existing
    {
        println!("File Not Found");
    }
    else 
    {
        let file_name = path_checker.file_name().unwrap().to_str().unwrap(); //Get filename from path
        let name = file_name.to_string(); //Convert to String
        let ext = path_checker.extension().unwrap().to_str(); //Get extension from path

        if ext != Some("wasm") && ext != Some("wat") //Check for valid file
        {
            println!("Invalid Format: Must be .wasm or .wat");
        }
        else //If valid file insert into list
        {
            //println!("Store Path and Filename");
            wasm_list.insert(WasmFile::new_wasm(name, trimmed_path.to_string()));  
            wasm_list.print();
            
            //Add method to write object to wasm_list.txt
        }
    }
    //Menu:
    //Call path_wasm agian to add another or return to main.
    let path_men: Vec<_> = vec![
        
        "Add wasm File",
        "Return to Main Menu",
    ];

    let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Add File With Path")
        .items(&path_men)
        .default(0)
        .interact()
        .unwrap();

    match choice {
        0 => path_wasm(wasm_list),
        1 => (),
            _ => unreachable!(),
    }
}
pub fn load_menu(wasm_list: &mut WasmList)
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
        1 => path_wasm(wasm_list),
        2 => (),
        _ => unreachable!(),
    }
}

