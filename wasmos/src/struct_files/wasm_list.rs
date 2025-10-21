use dialoguer::{Select, theme::ColorfulTheme};
use super::wasm_struct::WasmFile;
pub struct WasmList
{ //Stores a Structure WasmFile which holds information about .wasm files.
    head: Option<Box<WasmNode>>
}
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
            head: None
        }
    }
    pub fn insert(&mut self, node: WasmFile) //Node Insertion Function. Inserts files alphabetically
    {   
        let menu: Vec<_> = vec!["Yes","No"]; //Menu

        if self.head.is_none() || node.name < self.head.as_ref().unwrap().wasm_file.name //Case for no head or new nodes before head
        {
            let new_node = Box::new(WasmNode::new_node(node, self.head.take()));
            self.head = Some(new_node);
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
                let next = current.next.as_mut().unwrap();
                if next.wasm_file.name > node.name //Insert Node
                {
                    let new_node = Box::new(WasmNode::new_node(node, current.next.take()));
                    current.next = Some(new_node);
                    return;
                }
                else if next.wasm_file.name == node.name //If new node name is already in list ask to replace
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
                            next.wasm_file = node;
                            return
                        },
                        1 => return,
                        _ => unreachable!(),
                    }
                }
                current = current.next.as_mut().unwrap(); //Traverse node
            }
            current.next = Some(Box::new(WasmNode::new_node(node, None))); //Create the new node at end of the list
        }
    }
    pub fn delete(&mut self, name: String) //Function for removing the node from the list
    {
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

    }
}
