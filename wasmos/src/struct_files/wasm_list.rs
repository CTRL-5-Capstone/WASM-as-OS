use dialoguer::{Select, theme::ColorfulTheme};
use super::wasm_struct::WasmFile;
use std::fs;

pub fn delete_from_file(count: u8)
{
    let mut newlines: u8 = 0;
    let mut to_file = String::new();
    let from_file = fs::read_to_string( "./src/wasm_files/wasm_list.csv").expect("ERROR: Path to wasm_list.csv not found");
    for itter in from_file.chars()
    {
        if newlines != count
        {
                to_file.push(itter);           
        }  
        if itter == '\n'
        {
            newlines += 1;
        } 
    }
    fs::write("./src/wasm_files/wasm_list.csv", to_file).expect("ERROR: Path to wasm_list.csv not found");
}
fn add_to_file(count: u8, to_insert: String)
{
    let mut lines: u8 = 0;
    let from_file: String = fs::read_to_string("./src/wasm_files/wasm_list.csv").expect("ERROR: Path to wasm_list.csv not found");
    let mut to_file =  String::new();
    let mut before_file = String::new();
    if from_file.is_empty()
    {
        to_file = to_insert;
    }
    else
    {
        for itter in from_file.chars()
        {
            if lines == count
            {
                to_file = format!("{}{}", before_file, to_insert);
                lines += 1;
            }
            if lines < count
            {
                before_file.push(itter);
            }
            else
            {
                to_file.push(itter);
            }
            if itter == '\n'
            {
                lines += 1
            }
        }
    }
    fs::write("./src/wasm_files/wasm_list.csv", to_file).expect("ERROR: Path to wasm_list.csv not found");
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
        let mut count: u8 = 0;
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
            self.print();
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
            let mut current = self.head.as_mut().unwrap();
            while current.next.is_some() //Iterate thorugh list until spot found where new node name is less than the next node
            {
                count += 1;
                let next = current.next.as_mut().unwrap();
                if next.wasm_file.name >= node.name //Insert Node
                {
                    break;
                }
                current = current.next.as_mut().unwrap(); //Traverse node
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
        let mut count: u8 = 0;
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
