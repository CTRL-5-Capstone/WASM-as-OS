//Dependacies
use dialoguer::{Select, theme::ColorfulTheme};
use std::sync::mpsc::Sender;
use crate::struct_files::wasm_list::*;
use crate::run_wasm::wasm_control::Messages;

pub fn remove_wasm(wasm_list: &mut WasmList, index: usize, to_thread: Sender<Messages>) //Make function to remove WasmFile object from WasmList and wasm_list.txt
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
            to_thread.send(Messages::Delete(file_list[choice].clone())).expect("Critical Error Thread Unresponsive");
            wasm_list.delete(file_list[choice].clone());
            remove_wasm(wasm_list, choice - 1, to_thread.clone()); //Dev Note: Remove recursion and make this better
                                                      //Use a vec of refs and a loop instead?
        }
    }
}
pub fn cleanup_wasms(wasm_list: &mut WasmList)
{
    let to_stop = wasm_list.list_runningvec().0;
    //Add function to stop wasms

    for wasm in to_stop
    {
        wasm_list.running_false(wasm);
    }
}