//Dependacies
use dialoguer::{Select, theme::ColorfulTheme};
use std::io::{self, Write};
use crate::struct_files::wasm_list::*;

pub fn remove_wasm(wasm_list: &mut WasmList, index: usize) //Make function to remove WasmFile object from WasmList and wasm_list.txt
{   
    
    let mut file_list = wasm_list.list_namevec(); //Load Vec for dynamic delete menu
    if file_list.is_empty() //Return if no wasm files have been loaded
    {
        println!("No files are loaded");
    }
    else 
    {    
        file_list.insert(0, String::from("Return to main menu"));
        let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Delete a wasm file")
        .items(&file_list)
        .default(index)
        .interact()
        .unwrap();
        if choice == 0 {}
        else 
        {
            wasm_list.delete(file_list[choice].clone());
            remove_wasm(wasm_list, choice - 1);    
        }
    }
}