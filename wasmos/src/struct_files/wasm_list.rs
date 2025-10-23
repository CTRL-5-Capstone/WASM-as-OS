use dialoguer::{Select, theme::ColorfulTheme};
use super::wasm_struct::WasmFile;
use std::fs;

pub fn delete_from_file(count: u16)
{
    let mut file_lines: Vec<String> = Vec::new(); //Vec for storing file lines
    let mut to_file = String::new();
    let from_file = fs::read_to_string( "wasm_files/wasm_list.csv").expect("ERROR: Path to wasm_list.csv not found");
    if from_file.is_empty() //If file is empty return won't be possible or seen by user in final version will be stopped earlier
    {
        println!("File is empty!")
    }
    else
    {
        //Split lines from file into vector by \n
        file_lines = from_file.split('\n').map(|a_line| a_line.to_string()).collect();
        //Remove the file at count
        file_lines.remove(count as usize);
        //Rejoin lines
        to_file = file_lines.join("\n");
        to_file = to_file.trim().to_string(); //Trim any \n just in case
        fs::write("wasm_files/wasm_list.csv", to_file).expect("ERROR: Path to wasm_list.csv not found");
    }
}
fn add_to_file(count: u16, to_insert: String)
{
    let new_line = to_insert.trim().to_string(); //Remove white spaces from new line
    let mut to_file = String::new();
    let mut file_lines: Vec<String> = Vec::new(); //Vec for storing file lines
    let from_file: String = fs::read_to_string("wasm_files/wasm_list.csv").expect("ERROR: Path to wasm_list.csv not found").trim().to_string();
    if from_file.is_empty()
    {
        to_file = new_line; //if no lines in file insert new line
    }
    else {
        //If lines in file
        file_lines = from_file.split('\n').map(|a_line| a_line.to_string()).collect(); //Divide file into vec by \n
        file_lines.insert(count as usize , new_line); //Insert the new line
        to_file = file_lines.join("\n").trim().to_string(); //Concatonate lines in vec

    }
    fs::write("wasm_files/wasm_list.csv", to_file).expect("ERROR: Path to wasm_list.csv not found");
}
pub struct WasmList
{ //Stores a Structure WasmFile which holds information about .wasm files.
    head: Option<Box<WasmNode>>,
    tail: Option<Box<WasmNode>>
}
#[derive(Clone)]
struct WasmNode
{ //Node for WasmList
    wasm_file: WasmFile, //.wasm file structure
    next: Option<Box<WasmNode>>
}
impl WasmNode
{
    pub fn new_node(wasm_file: WasmFile, next: Option<Box<WasmNode>>) -> WasmNode
    { //New Node "Constructor"
        WasmNode
        {
            wasm_file,
            next
        }
    }
}
impl WasmList
{
    pub fn new_list() -> WasmList //default constructor
    {
        WasmList
        {
            head: None,
            tail: None
        }
    }
    pub fn insert(&mut self, node: WasmFile) //Node Insertion Function. Inserts files alphabetically
    {   
        let menu: Vec<_> = vec!["Yes","No"]; //Menu
        let mut count: u16 = 0;
        let to_insert = format!("{}, {}\n", node.name, node.path_to); //To be inserted into wasm_list.csv

        if self.head.is_none() || self.head.as_ref().unwrap().wasm_file.name > node.name //Case for no head or new nodes before head
        {
            let new_node = Box::new(WasmNode::new_node(node, self.head.clone()));
            self.head = Some(new_node);
            if self.head.as_mut().unwrap().next.is_none()
            {
                self.tail = self.head.clone();
            }
            add_to_file(count, to_insert);
        }
        else if self.head.as_ref().unwrap().wasm_file.name == node.name //Case for node equal to head
        {
            let choice = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("File with that name already exists, replace it?")
                .items(&menu)
                .default(0)
                .interact()
                .unwrap();
            match choice {
                0 => 
                {
                    self.head.as_mut().unwrap().wasm_file = node;
                    delete_from_file(count);
                    add_to_file(count, to_insert);
                },
                1 => (),
                _ => unreachable!(),
            }
        }
        else //Case for node in list
        {
            count += 1;
            let mut current = self.head.as_mut().unwrap();
            while current.next.is_some() //Iterate thorugh list until spot found where new node name is less than the next node
            {
                println!("{}", count);
                let next = current.next.as_mut().unwrap();
                if next.wasm_file.name >= node.name //Insert Node
                {
                    break;
                }
                current = current.next.as_mut().unwrap(); //Traverse node
                count += 1;
            }
            if current.next.is_none()
            {
                current.next = Some(Box::new(WasmNode::new_node(node, None))); //Create the new node at end of the list
                self.tail = current.next.clone();
                add_to_file(count, to_insert);
            }
            else if current.next.as_ref().unwrap().wasm_file.name == node.name //If new node name is already in list ask to replace
            {
                let choice = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("File with that name already exists, replace it?")
                    .items(&menu)
                    .default(0)
                    .interact()
                    .unwrap();
                match choice {
                    0 => {
                        current.next.as_mut().unwrap().wasm_file = node;
                        delete_from_file(count);
                        add_to_file(count, to_insert);
                    }
                    1 => (),
                    _ => unreachable!(),
                }
            }
            else 
            {
                    let new_node = Box::new(WasmNode::new_node(node, current.next.take()));
                    current.next = Some(new_node); 
                    add_to_file(count, to_insert);
            }
        }


    }
    pub fn delete(&mut self, name: String) //Function for removing the node from the list
    {
        let mut count: u16 = 0;
        let menu: Vec<_> = vec!["Yes","No"];
        let choice = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Delete File?")
            .items(&menu)
            .default(0)
            .interact()
            .unwrap();
        match choice 
        {
            0=> (),
            1 => return,
            _ => unreachable!(),
        }
        if self.head.is_none() //Stops panic if list is empty
        {
            println!("\nData Corruption, File Not Found\n")
        }
        else if self.head.as_ref().unwrap().wasm_file.name == name //Case for deleting the head node
        {
            let new_head = self.head.as_mut().unwrap().next.take();
            self.head = new_head;
        }
        else 
        {
            let mut current = self.head.as_mut().unwrap(); //Set Traversing node to head.
            loop 
            {
                count += 1;
                if current.next.is_none() //Case for node not found
                {
                    println!("\nData Corruption, File Not Found\n");
                    return;
                }
                let next = current.next.as_mut().unwrap();
                if next.wasm_file.name == name //Case for node to delete found
                {
                    break;
                }    
                current = current.next.as_mut().unwrap();
            }   
            //Delete node
            let next = current.next.as_mut().unwrap();
            let new_next = next.next.take();
            current.next = new_next;
            
        }
        delete_from_file(count);
    }
    pub fn print(&mut self)
    {
        let mut current = self.head.as_ref();
        while let Some(node) = current
        {
            println!("{}", node.wasm_file.name);
            current = node.next.as_ref();

        }
    }
}
