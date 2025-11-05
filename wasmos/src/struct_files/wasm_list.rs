use dialoguer::{Select, theme::ColorfulTheme};
use super::wasm_struct::WasmFile;
use std::{cell::RefCell, fs, rc::Rc};

pub fn delete_from_file(count: u16)
{
    let mut from_file = fs::read_to_string( "wasm_files/wasm_list.csv").expect("ERROR: Path to wasm_list.csv not found");
    if from_file.is_empty() //If file is empty return won't be possible or seen by user in final version will be stopped earlier
    {
        println!("File is empty!")
    }
    else
    {
        //Split lines from file into vector by \n
        let mut file_lines: Vec<String> = from_file.split('\n').map(|a_line| a_line.to_string()).collect();
        //Remove the file at count
        file_lines.remove(count as usize);
        //Rejoin lines
        from_file = file_lines.join("\n");
        from_file = from_file.trim().to_string(); //Trim any \n just in case
        fs::write("wasm_files/wasm_list.csv", from_file).expect("ERROR: Path to wasm_list.csv not found");
    }
}
fn add_to_file(count: u16, to_insert: String)
{
    let new_line = to_insert.trim().to_string(); //Remove white spaces from new line 
    let mut from_file: String = fs::read_to_string("wasm_files/wasm_list.csv").expect("ERROR: Path to wasm_list.csv not found").trim().to_string();
    if from_file.is_empty()
    {
       from_file = new_line; //if no lines in file insert new line
    }
    else {
        //If lines in file
        let mut file_lines: Vec<String> = from_file.split('\n').map(|a_line| a_line.to_string()).collect(); //Divide file into vec by \n
        file_lines.insert(count as usize , new_line); //Insert the new line
        from_file = file_lines.join("\n").trim().to_string(); //Concatonate lines in vec

    }
    fs::write("wasm_files/wasm_list.csv", from_file).expect("ERROR: Path to wasm_list.csv not found");
}
pub struct WasmList
{ //Stores a Structure WasmFile which holds information about .wasm files.
    head: Option<Rc<RefCell<WasmNode>>>,
    tail: Option<Rc<RefCell<WasmNode>>>
}
#[derive(Clone)]
pub struct WasmNode
{ //Node for WasmList
    wasm_file: WasmFile, //.wasm file structure
    next: Option<Rc<RefCell<WasmNode>>>
}
impl WasmNode
{
    pub fn new_node(wasm_file: WasmFile, next: Option<Rc<RefCell<WasmNode>>>) -> WasmNode
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
    pub fn append_list(&mut self, node: WasmFile)
    {
        match self.tail.clone()
        {
            None =>
            {
                self.head = Some(Rc::new(RefCell::new(WasmNode::new_node(node, None))));
                self.tail = self.head.clone();
            }
            Some(old_node) =>
            {
                let new_node = Some(Rc::new(RefCell::new(WasmNode::new_node(node, None))));
                old_node.borrow_mut().next = new_node.clone();
                self.tail = new_node.clone();
            }
        }
    }
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
        let to_insert = format!("{},{}\n", node.name, node.path_to); //To be inserted into wasm_list.csv

        match self.head.clone()
        {
            None =>
            {
                self.append_list(node);
                add_to_file(count, to_insert);
            }

            Some(head_node) =>
            {  
                let head_name = head_node.borrow().wasm_file.name.clone();
                if head_name > node.name
                {
                    let new_node = Rc::new(RefCell::new(WasmNode::new_node(node, Some(head_node.clone()))));
                    self.head = Some(new_node);
                    add_to_file(count, to_insert);
                }
                else if head_name == node.name
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
                            head_node.borrow_mut().wasm_file = node;
                            delete_from_file(count);
                            add_to_file(count, to_insert);
                        },
                        1 => (),
                        _ => unreachable!(),
                    }           
                }
                else
                {
                    let mut current = head_node.clone();
                    let mut next = current.borrow().next.clone();
                    loop
                    {
                        count += 1;
                        match next.clone()
                        {
                            None =>
                            {
                                self.append_list(node);
                                add_to_file(count, to_insert);
                                return;
                            }
                            Some(list_node) =>
                            {
                                let curr_name = list_node.borrow().wasm_file.name.clone();
                                if curr_name == node.name
                                {
                                    let choice = Select::with_theme(&ColorfulTheme::default())
                                        .with_prompt("File with that name already exists, replace it?")
                                        .items(&menu)
                                        .default(0)
                                        .interact()
                                        .unwrap();
                                    match choice 
                                    {
                                        0 => 
                                        {
                                            list_node.borrow_mut().wasm_file = node;
                                            delete_from_file(count);
                                            add_to_file(count, to_insert);
                                            break;
                                        }
                                        1 => (),
                                        _ => unreachable!(),
                                    }
                                } 
                                else if curr_name > node.name
                                {
                                    current.borrow_mut().next = Some(Rc::new(RefCell::new(WasmNode::new_node(node, Some(list_node.clone())))));
                                    add_to_file(count, to_insert);
                                    break;
                                }
                                else
                                {
                                    current = list_node;
                                    next = current.borrow().next.clone();
                                }
                            }


                        }
                    }
                }
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
        match self.head.clone()
        {
            None => 
            {
                println!("\nData Corruption, File Not Found\n");
            }
            Some(head_node) =>
            {
                let head_name = head_node.borrow().wasm_file.name.clone();
                if head_name == name //Case for deleting the head node
                {
                    self.head = head_node.borrow().next.clone();
                    if self.head.is_none()
                    {
                        self.tail = None;
                    }
                }
                else 
                {
                    let mut current = head_node; //Set Traversing node to head.
                    loop 
                    {
                        count += 1;
                        let next = current.borrow().next.clone();
                        match next
                        {
                            None =>
                            {
                                println!("\nData Corruption, File Not Found\n");
                                return;
                            }
                            Some(node) =>
                            {
                                if node.borrow().wasm_file.name == name //Case for node to delete found
                                {
                                    current.borrow_mut().next = node.borrow().next.clone();
                                    if current.borrow().next.is_none()
                                    {
                                        self.tail = Some(current);
                                    }
                                    break;
                                }   
                                current = node.clone();
                            }
                        }
                    }   
                    //Delete node
    
                    
                }   
                delete_from_file(count);
            }
        }
    }
    pub fn print(&mut self)
    {
        let mut current = self.head.clone();
        loop
        {   
            match current
            {
                None => 
                {
                    break;
                }
                Some(node) =>
                {
                    println!("{}", node.borrow().wasm_file.name.clone());
                    current = node.borrow().next.clone();
                }
            } 
        }
    }   
    pub fn list_namevec(&mut self) -> Vec<String>
    {
        let mut name_vec: Vec<String> = Vec::new();
        let mut current = self.head.clone();
        loop
        {
            match current
            {
                None => {break;}
                Some(node) =>
                {
                    name_vec.push(node.borrow().wasm_file.name.clone());
                    current = node.borrow().next.clone();
                }
            }
        }
        name_vec
    }
    pub fn list_runningvec(&mut self) -> (Vec<Rc<RefCell<WasmNode>>>, Vec<String>) 
    {            
        let mut running_vec: Vec<String> = Vec::new();
        let mut wasm_vec: Vec<Rc<RefCell<WasmNode>>> = Vec::new();
        let mut current = self.head.clone();
        loop
        {
            match current
            {
                None => {break;}
                Some(node) =>
                {
                    if node.borrow().wasm_file.running
                    {
                        running_vec.push(node.borrow().wasm_file.name.clone());
                        wasm_vec.push(node.clone());

                    }
                    current = node.borrow().next.clone();
                }
            }
        }
        (wasm_vec, running_vec)
    }
    pub fn list_haltedvec(&mut self) -> (Vec<Rc<RefCell<WasmNode>>>, Vec<String>) 
    {            
        let mut nonrunning_vec: Vec<String> = Vec::new();
        let mut wasm_vec: Vec<Rc<RefCell<WasmNode>>> = Vec::new();
        let mut current = self.head.clone();
        loop
        {
            match current
            {
                None => {break;}
                Some(node) =>
                {
                    if !node.borrow().wasm_file.running
                    {
                        nonrunning_vec.push(node.borrow().wasm_file.name.clone());
                        wasm_vec.push(node.clone());

                    }
                    current = node.borrow().next.clone();
                }
            }
        }
        (wasm_vec, nonrunning_vec)
    }
    pub fn running_false(&mut self, node: Rc<RefCell<WasmNode>>)
    {
        node.borrow_mut().wasm_file.running = false;
    }
    pub fn running_true(&mut self, node: Rc<RefCell<WasmNode>>)
    {
        node.borrow_mut().wasm_file.running = true;
    }
    /* could be a useful function not needed yet
    pub fn get_file(&mut self, name: String)
    {

    }
    */
}