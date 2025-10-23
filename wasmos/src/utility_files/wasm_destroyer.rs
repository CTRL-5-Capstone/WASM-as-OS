//Dependacies
use dialoguer::{Select, theme::ColorfulTheme};
use std::io::{self, Write};
use crate::struct_files::wasm_list::*;

pub fn remove_wasm(wasm_list: &mut WasmList) //Make function to remove WasmFile object from WasmList and wasm_list.txt
{   
    
    //Temp to test add a robust menu later with dialoguer or rs_menu?
    print!("\nPlease enter the name of the file you wish to delete: ");
    io::stdout().flush().expect("Failure to Flush wasm_destroyer");
    let mut to_delete = String::new();
    io::stdin().read_line( &mut to_delete).expect("Path Input ERROR");
    to_delete = to_delete.trim().to_string();
    wasm_list.delete(to_delete);
    wasm_list.print();

}