use dialoguer::{Select, theme::ColorfulTheme};
use crate::struct_files::wasm_list::*;
use super::wasm_engine::wasm_engine;
pub fn start_wasm(wasm_list: &mut WasmList)
{
    let wasm_tup = wasm_list.list_haltedvec();
    let mut wasm_vec = wasm_tup.0;
    let mut halted_vec = wasm_tup.1;
    let mut choice = 0;
    if halted_vec.is_empty()
    {
        println!("No running wasm modules")
    }
    else
    {
        halted_vec.insert(0, String::from("Return to main menu"));
        loop
        {    
            if halted_vec.len() == 1 {break;}
            choice = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Stop a wasm file")
            .items(&halted_vec)
            .default(choice)
            .interact()
            .unwrap();
            if choice == 0 {break;}
            else 
            {
                //Stop a wasm file here or from a function
                wasm_list.running_true(wasm_vec[choice - 1].clone()); //Set wasm to running
                wasm_engine();
                halted_vec.remove(choice);
                wasm_vec.remove(choice - 1);
                choice -= 1;
            }
        }
    }
}
pub fn halt_wasm(wasm_list: &mut WasmList)
{
    let wasm_tup = wasm_list.list_runningvec();
    let mut wasm_vec = wasm_tup.0;
    let mut started_vec = wasm_tup.1;
    let mut choice = 0;
    
    if started_vec.is_empty()
    {
        println!("No running wasm modules")
    }
    else
    {
        started_vec.insert(0, String::from("Return to main menu"));
        loop
        {    
            if started_vec.len() == 1 {break;}
            choice = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Stop a wasm file")
            .items(&started_vec)
            .default(choice)
            .interact()
            .unwrap();
            if choice == 0 {break;}
            else 
            {
                //Stop a wasm file here or from a function
                wasm_list.running_false(wasm_vec[choice - 1].clone()); //Set wasm to stopped
                started_vec.remove(choice);
                wasm_vec.remove(choice - 1);
                choice -= 1; 
            }
        }
    }
}