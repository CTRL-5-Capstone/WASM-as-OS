use dialoguer::{Select, theme::ColorfulTheme};
use crate::struct_files::wasm_list::*;
pub fn start_wasm(wasm_list: &mut WasmList, index: usize)
{
    let mut halted_vec: Vec<String> = wasm_list.list_notrunningvec();
    if halted_vec.is_empty()
    {
        println!("No wasm files to run")
    }
    else
    {
        halted_vec.insert(0, String::from("Return to main menu"));
        let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Start a wasm file")
        .items(&halted_vec)
        .default(index)
        .interact()
        .unwrap();
        if choice == 0 {}
        else 
        {
            //Implement function to run a wasm module.
            wasm_list.running_true(halted_vec[choice].clone()); //Sets wasm to running
            start_wasm(wasm_list, choice - 1); //recursive menu call    
        }
    }
}
pub fn halt_wasm(wasm_list: &mut WasmList, index: usize)
{
    let mut started_vec: Vec<String> = wasm_list.list_runningvec();
    if started_vec.is_empty()
    {
        println!("No running wasm modules")
    }
    else
    {
        started_vec.insert(0, String::from("Return to main menu"));
        let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Stop a wasm file")
        .items(&started_vec)
        .default(index)
        .interact()
        .unwrap();
        if choice == 0 {}
        else 
        {
            //Stop a wasm file here or from a function
            wasm_list.running_false(started_vec[choice].clone()); //Set wasm to stopped
            halt_wasm(wasm_list, choice - 1); //recursive menu call
        }
    }
}