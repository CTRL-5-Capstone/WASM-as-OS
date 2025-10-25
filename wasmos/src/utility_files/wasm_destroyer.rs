//Dependacies
use dialoguer::{Select, theme::ColorfulTheme};
use std::io::{self, Write};
use crate::struct_files::wasm_list::*;

pub fn remove_wasm(wasm_list: &mut WasmList, index: usize) //Make function to remove WasmFile object from WasmList and wasm_list.txt
{   
    
    //Temp to test add a robust menu later with dialoguer or rs_menu?
    /*print!("\nPlease enter the name of the file you wish to delete: ");
    io::stdout().flush().expect("Failure to Flush wasm_destroyer");
    let mut to_delete = String::new();
    io::stdin().read_line( &mut to_delete).expect("Path Input ERROR");
    to_delete = to_delete.trim().to_string();
    wasm_list.delete(to_delete);
    wasm_list.print();
    */
    //Start of new menu
    let mut file_list = wasm_list.list_namevec();
    if file_list.is_empty()
    {
        println!("No files are loaded");
    }
    else 
    {    
        file_list.insert(0, String::from("Return to main menu"));
        let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("WasmAsOS")
        .items(&file_list)
        .default(index)
        .interact()
        .unwrap();
        if choice == 0 {}
        else 
        {
            wasm_list.delete(file_list[choice].clone());
            remove_wasm(wasm_list, choice);    
        }
    }
}