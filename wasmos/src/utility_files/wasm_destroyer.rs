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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_wasm::wasm_control::Messages;
    use std::sync::mpsc::channel;

    #[test]
    fn test_channel_sender_cloneable_for_remove_wasm() {
        let (tx, rx) = channel::<Messages>();
        let tx_cloned = tx.clone();
        tx_cloned
            .send(Messages::Delete("test".to_string()))
            .expect("Failed to send message through cloned sender");
        match rx.recv() {
            Ok(Messages::Delete(name)) => assert_eq!(name, "test"),
            Ok(_) => panic!("Received the wrong message type"),
            Err(e) => panic!("Failed to receive message: {}", e),
        }
    }

    #[test]
    fn test_delete_message_carries_correct_name() {
        let (tx, rx) = channel::<Messages>();
        tx.send(Messages::Delete("my_wasm".to_string())).unwrap();
        match rx.recv().unwrap() {
            Messages::Delete(n) => assert_eq!(n, "my_wasm"),
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_multiple_senders_all_deliver() {
        let (tx, rx) = channel::<Messages>();
        let tx2 = tx.clone();
        let tx3 = tx.clone();

        tx.send(Messages::Delete("wasm_1".to_string())).unwrap();
        tx2.send(Messages::Delete("wasm_2".to_string())).unwrap();
        tx3.send(Messages::Delete("wasm_3".to_string())).unwrap();

        let mut received = vec![];
        for _ in 0..3 {
            match rx.recv().unwrap() {
                Messages::Delete(n) => received.push(n),
                _ => panic!("Wrong message type"),
            }
        }

        assert!(received.contains(&"wasm_1".to_string()));
        assert!(received.contains(&"wasm_2".to_string()));
        assert!(received.contains(&"wasm_3".to_string()));
    }

    #[test]
    fn test_delete_sent_before_list_remove() {
        // Verifies the order: Delete message sent FIRST, then list.delete()
        // This mirrors what remove_wasm() does in wasm_destroyer.rs
        let (tx, rx) = channel::<Messages>();
        let mut list = WasmList::new_list();

        let name = "doomed_wasm".to_string();

        // Step 1: send delete to thread first
        tx.send(Messages::Delete(name.clone())).unwrap();

        // Step 2: then remove from list
        list.delete(name.clone());

        // Verify thread got the message
        match rx.recv().unwrap() {
            Messages::Delete(n) => assert_eq!(n, "doomed_wasm"),
            _ => panic!("Expected Delete message"),
        }

        // Verify list no longer has it
        let names = list.list_namevec();
        assert!(!names.contains(&name));
    }
}