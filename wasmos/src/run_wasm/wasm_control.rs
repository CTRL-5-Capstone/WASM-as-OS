use dialoguer::{Select, theme::ColorfulTheme};
use crate::struct_files::wasm_list::*;
pub fn start_wasm(&wasm_list: WasmList, index: usize)
{
    let mut halted_vec: Vec<String> = wasm_list.running_vec();
    if halted_vec.is_empty()
    {
        println("No running wasm modules")
    }
    else
    {
        file_list.insert(0, String::from("Return to main menu"));
        let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Start a wasm file")
        .items(&file_list)
        .default(index)
        .interact()
        .unwrap();
        if choice == 0 {}
        else 
        {
            //Implement function to run a wasm module.
            start_wasm(wasm_list, choice - 1);    
        }
    }
}
pub fn halt_wasm(&wasm_list: WasmList, index: usize)
{
    let mut halted_vec: Vec<String> = wasm_list.nonrunning_vec();
    if halted_vec.is_empty()
    {
        println("No running wasm modules")
    }
    else
    {
        file_list.insert(0, String::from("Return to main menu"));
        let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Stop a wasm file")
        .items(&file_list)
        .default(index)
        .interact()
        .unwrap();
        if choice == 0 {}
        else 
        {
            //Stop a wasm file here or from a function
            halt_wasm(wasm_list, choice - 1);    
        }
    }
}