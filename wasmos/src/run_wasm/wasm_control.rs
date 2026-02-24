use crate::run_wasm::build_runtime::Runtime;
use dialoguer::{Select, theme::ColorfulTheme};
use crate::struct_files::wasm_list::*;
use std::path::Path;
use std::time;
use std::thread::sleep;
use std::time::Duration;
use std::sync::mpsc::{Receiver, Sender};
pub enum Messages
{
    Flog(String, bool),
    Clog(String, bool),
    Start(Runtime),
    Pause(String),
    Resume(String),
    Stop(String),
    Limit(String, usize, bool),
    Delete(String),
}
pub fn start_wasm_by_id(wasm_list: &mut WasmList, id: &str) {
    let wasm_tup = wasm_list.list_haltedvec();
    let wasm_vec = wasm_tup.0;
    
    for wasm in wasm_vec {
        if  wasm.lock().unwrap().wasm_file.name == id {
            let path_name = wasm.lock().unwrap().wasm_file.path_to.clone();
            let path = Path::new(&path_name);
                wasm_list.running_true(wasm.clone());
            }
    }
}

pub fn halt_wasm_by_id(wasm_list: &mut WasmList, id: &str) -> bool {
    let wasm_tup = wasm_list.list_runningvec();
    let wasm_vec = wasm_tup.0;
    
    for wasm in wasm_vec {
        if wasm.lock().unwrap().wasm_file.name == id {
            wasm_list.running_false(wasm.clone());
            return true;
        }
    }
    false
}
pub fn new_runtime_options(mut runtime: Runtime, to_thread: Sender<Messages>)
{
    let true_arr = [
        "Start Runtime",
        "Disable Limit",
        "Disable Console Logging",
        "Disable File Logging",
        "Disable Paused From Start",
    ];
    let false_arr = [
        "Start Runtime",
        "Enable Limit",
        "Enable Console Logging",
        "Enable File Logging",
        "Enable Paused From Start",
    ];
    let larr = [
        "Custom",
        "500",
        "1000",
        "2500",
        "5000",
    ];
    let mut menu_arr:[&str; 5] = false_arr;
    let mut choice = 0;
    loop{
        choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Runtime Options")
        .items(&menu_arr)
        .default(choice)
        .interact()
        .unwrap();
        match choice{
            0 => {
                to_thread.send(Messages::Start(runtime)).expect("Critical Error Thread Unresponsive");
                break;
            }
            1 => {
                if !runtime.limflag
                {
                    let lchoice = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Runtime Options")
                    .items(&larr)
                    .default(0)
                    .interact()
                    .unwrap();
                    match lchoice{
                        0 => {
                            let mut slimit = String::new();
                            std::io::stdin().read_line(&mut slimit).expect("Limit Input Error New Runtime Options");
                            if let Ok(limit) = slimit.trim().parse::<usize>()
                            {
                                runtime.limit = limit;
                            }
                            else {
                                println!("Invalid Input");
                                continue;
                            }
                        }
                        1 => runtime.limit = 500,
                        2 => runtime.limit = 1000,
                        3 => runtime.limit = 2500,
                        4 => runtime.limit = 5000,
                        _ => unreachable!(),
                    }
                        menu_arr[1] = true_arr[1];
                        runtime.limflag = true;
                }
                else {
                    runtime.limflag = false;
                    menu_arr[1] = false_arr[1];
                    runtime.limit = 0;
                }
            }
            2 => {
                if runtime.clog
                {
                    menu_arr[2] = false_arr[2];
                    runtime.clog = false;
                }
                else
                {
                    menu_arr[2] = true_arr[2];
                    runtime.clog = true;
                }
            } 
            3 => {
                if runtime.flog
                {
                    menu_arr[3] = false_arr[3];
                    runtime.flog = false;
                }
                else
                {
                    menu_arr[3] = true_arr[3];
                    runtime.flog = true;
                }
            }
            4 => {
                if runtime.paused
                {
                    menu_arr[4] = false_arr[4];
                    runtime.paused = false;
                }
                else
                {
                    menu_arr[4] = true_arr[4];
                    runtime.paused = true;
                }
            }
            _ => unreachable!(),
        }
    }
}
pub fn runtime_options(mut runtime: Runtime, to_thread: Sender<Messages>, running: bool)
{
    let true_arr = [
        "Return to Edit Runtimes",
        "Disable Limit",
        "Disable Console Logging",
        "Disable File Logging",
    ];
    let false_arr = [
        "Return to Edit Runtimes",
        "Enable Limit",
        "Enable Console Logging",
        "Enable File Logging",
    ];
    let larr = [
        "Custom",
        "500",
        "1000",
        "2500",
        "5000",
    ];
    let mut menu_arr:[&str; 4] = false_arr;
    let mut choice = 0;
    loop{
        choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Runtime Options")
        .items(&menu_arr)
        .default(choice)
        .interact()
        .unwrap();
        match choice{
            0 => {
                to_thread.send(Messages::Start(runtime)).expect("Critical Error Thread Unresponsive");
                break;
            }
            1 => {
                if !runtime.limflag
                {
                    let lchoice = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Runtime Options")
                    .items(&larr)
                    .default(0)
                    .interact()
                    .unwrap();
                    match lchoice{
                        0 => {
                            let mut slimit = String::new();
                            std::io::stdin().read_line(&mut slimit).expect("Limit Input Error Runtime Options");
                            if let Ok(limit) = slimit.trim().parse::<usize>()
                            {
                                runtime.limit = limit;
                            }
                            else {
                                println!("Invalid Input");
                                continue;
                            }
                        }
                        1 => runtime.limit = 500,
                        2 => runtime.limit = 1000,
                        3 => runtime.limit = 2500,
                        4 => runtime.limit = 5000,
                        _ => unreachable!(),
                    }
                        menu_arr[1] = true_arr[1];
                        runtime.limflag = true;
                        if running
                        {
                            to_thread.send(Messages::Limit(runtime.module.name.clone(), runtime.limit, true)).expect("Critical Error Thread Unresponsive");
                        }
                }
                else {
                    runtime.limflag = false;
                    menu_arr[1] = false_arr[1];
                    runtime.limit = 0;
                    if running
                    {
                        to_thread.send(Messages::Limit(runtime.module.name.clone(), 0, false)).expect("Critical Error Thread Unresponsive");
                    }
                }
            }
            2 => {
                if runtime.clog
                {
                    menu_arr[2] = false_arr[2];
                    runtime.clog = false;
                }
                else
                {
                    menu_arr[2] = true_arr[2];
                    runtime.clog = true;
                }
                if running
                {
                    to_thread.send(Messages::Clog(runtime.module.name.clone(), runtime.clog)).expect("Critical Error Thread Unresponsive");
                }
            } 
            3 => {
                if runtime.flog
                {
                    menu_arr[3] = false_arr[3];
                    runtime.flog = false;
                }
                else
                {
                    menu_arr[3] = true_arr[3];
                    runtime.flog = true;
                }
                if running
                {
                    to_thread.send(Messages::Flog(runtime.module.name.clone(), runtime.flog)).expect("Critical Error Thread Unresponsive");
                }
            }
            _ => unreachable!(),
        }
    }
}
pub fn start_wasm(wasm_list: &mut WasmList, to_thread: Sender<Messages>)
{
    let wasm_tup = wasm_list.list_haltedvec();
    let mut wasm_vec = wasm_tup.0;
    let mut halted_vec = wasm_tup.1;
    let mut choice = 0;
    if halted_vec.is_empty()
    {
        println!("No wasm modules to run")
    }
    else
    {
        halted_vec.insert(0, String::from("Return to main menu"));
        loop
        {    
            if halted_vec.len() == 1 {break;}
            choice = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Start a wasm file")
            .items(&halted_vec)
            .default(choice)
            .interact()
            .unwrap();
            if choice == 0 {break;}
            else 
            {
                new_runtime_options(wasm_vec[choice - 1].lock().unwrap().wasm_file.runtime.clone(), to_thread.clone());
                wasm_list.running_true(wasm_vec[choice - 1].clone()); //Set wasm to running
                halted_vec.remove(choice);
                wasm_vec.remove(choice - 1);
                choice -= 1;
            }
        }
    }
}
pub fn pause_wasm(wasm_list: &mut WasmList, to_thread: Sender<Messages>)
{
    let wasm_tup = wasm_list.list_unpausedvec();
    let mut wasm_vec = wasm_tup.0;
    let mut started_vec = wasm_tup.1;
    let mut choice = 0;
    
    if started_vec.is_empty()
    {
        println!("No pauseable wasm modules")
    }
    else
    {
        started_vec.insert(0, String::from("Return to main menu"));
        loop
        {    
            if started_vec.len() == 1 {break;}
            choice = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Pause a wasm file")
            .items(&started_vec)
            .default(choice)
            .interact()
            .unwrap();
            if choice == 0 {break;}
            else 
            {
                //Stop a wasm file here or from a function
                wasm_vec[choice - 1].lock().unwrap().wasm_file.runtime.paused = true; //Set wasm to paused.
                let name = wasm_vec[choice - 1].lock().unwrap().wasm_file.name.clone();
                to_thread.send(Messages::Pause(name)).expect("Critical Error Thread Unresponsive.");
                started_vec.remove(choice);
                wasm_vec.remove(choice - 1);

                choice -= 1; 
            }
        }
    }    
}
pub fn unpause_wasm(wasm_list: &mut WasmList, to_thread: Sender<Messages>)
{
    let wasm_tup = wasm_list.list_pausevec();
    let mut wasm_vec = wasm_tup.0;
    let mut started_vec = wasm_tup.1;
    let mut choice = 0;
    
    if started_vec.is_empty()
    {
        println!("No unpauseable wasm modules")
    }
    else
    {
        started_vec.insert(0, String::from("Return to main menu"));
        loop
        {    
            if started_vec.len() == 1 {break;}
            choice = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Unpause a wasm file")
            .items(&started_vec)
            .default(choice)
            .interact()
            .unwrap();
            if choice == 0 {break;}
            else 
            {
                //Stop a wasm file here or from a function
                wasm_vec[choice - 1].lock().unwrap().wasm_file.runtime.paused = false; //Set wasm to paused.
                let name = wasm_vec[choice - 1].lock().unwrap().wasm_file.name.clone();
                to_thread.send(Messages::Resume(name)).expect("Critical Error Thread Unresponsive.");
                started_vec.remove(choice);
                wasm_vec.remove(choice - 1);

                choice -= 1; 
            }
        }
    }    
}
pub fn halt_wasm(wasm_list: &mut WasmList, to_thread: Sender<Messages>)
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
                let name = wasm_vec[choice - 1].lock().unwrap().wasm_file.name.clone();
                to_thread.send(Messages::Stop(name)).expect("Critical Error Thread Unresponsive.");
                started_vec.remove(choice);
                wasm_vec.remove(choice - 1);

                choice -= 1; 
            }
        }
    }
}
pub fn edit_runtimes(wasm_list: &mut WasmList, to_thread: Sender<Messages>)
{
    let mut runt: Runtime;
    let mut running: bool;
    let mut all_vec: Vec<String> = vec!["Return to main menu".to_string(), "Show only running wasms".to_string()];
    let mut running_vec: Vec<String> = vec!["Return to main menu".to_string(), "Show all wasms".to_string()];
    let mut choice = 0;
    let mut all = false;
    
    let runtup = wasm_list.list_runningvec();
    let running_wasms = runtup.0;
    running_vec.extend(runtup.1);

    let alltup = wasm_list.convert_vec();
    let all_wasm = alltup.0;
    all_vec.extend(alltup.1);

    let mut menu_vec = &running_vec;
    loop{
        choice = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Unpause a wasm file")
                .items(menu_vec)
                .default(choice)
                .interact()
                .unwrap();
                if choice == 0 {break;}
                else if choice == 1
                {
                    if all
                    {
                        choice = 0;
                        all = false;
                        menu_vec = &running_vec;
                    }
                    else
                    {
                        all = true;
                        menu_vec = &all_vec;
                    }
                }
                else 
                {
                    if all
                    {
                        runt = all_wasm[choice - 2].lock().unwrap().wasm_file.runtime.clone();
                        running = all_wasm[choice - 2].lock().unwrap().wasm_file.running;
                    }
                    else
                    {
                        runt = running_wasms[choice - 2].lock().unwrap().wasm_file.runtime.clone();
                        running = true;
                    }
                    runtime_options(runt, to_thread.clone(), running);
                }
        }

}
pub fn runtime_loop(msgto_main: Sender<Messages>, msgfrom_main: Receiver<Messages>)
{
    let mut activity;
    let mut runtime_wasms: Vec<Runtime> = Vec::new();
    let mut i = 0;
    loop
    {
        while let Ok(mssg) = msgfrom_main.try_recv()
        {
            match mssg{
                Messages::Start(rtime) => runtime_wasms.push(rtime),
                Messages::Stop(name) => {
                    while i <  runtime_wasms.len()
                    {
                        if runtime_wasms[i].module.name == name
                        {
                            runtime_wasms.remove(i);
                            break;
                        }
                        i += 1;
                    }
                }
                Messages::Clog(name, flag) => {
                    while i < runtime_wasms.len()
                    {
                        if runtime_wasms[i].module.name == name
                        {
                            runtime_wasms[i].clog = flag;
                            break;
                        }
                        i += 1;
                    }
                }
                Messages::Flog(name, flag) => {
                    while i < runtime_wasms.len()
                    {
                        if runtime_wasms[i].module.name == name
                        {
                            runtime_wasms[i].flog = flag;
                            break;
                        }
                        i += 1;
                    }
                }
                Messages::Pause(name) => {
                    while i < runtime_wasms.len()
                    {
                        if runtime_wasms[i].module.name == name
                        {
                            runtime_wasms[i].paused = true;
                            break;
                        }
                        i += 1;
                    }
                }
                Messages::Resume(name) =>
                {
                    while i < runtime_wasms.len()
                    {
                        if runtime_wasms[i].module.name == name
                        {
                            runtime_wasms[i].paused = false;
                            break;
                        }
                        i += 1;
                    }
                }
                Messages::Limit(name, limit, flag) => {
                    while i < runtime_wasms.len()
                    {
                        if runtime_wasms[i].module.name == name
                        {
                            runtime_wasms[i].limit = limit;
                            runtime_wasms[i].limflag = flag;
                            break;
                        }
                        i += 1;
                    }
                }
                Messages::Delete(name) => {
                    while i < runtime_wasms.len()
                    {
                        if runtime_wasms[i].module.name == name
                        {
                            runtime_wasms.remove(i);
                            break;
                        }
                        i += 1;
                    }

                }
                _ => panic!("Invalid Thread Communication."),
                
            }
            sleep(Duration::from_millis(300));
            i = 0;
        }
        
        
        activity = false;
        while i < runtime_wasms.len()
        {
            if !runtime_wasms[i].paused
            {
                if runtime_wasms[i].limflag
                {
                    for _j in 0..runtime_wasms[i].priority
                    {
                        if runtime_wasms[i].limit > 0
                        {
                            runtime_wasms[i].run_prog();
                            runtime_wasms[i].limit -= 1;
                            activity = true;
                        }
                        else{
                            runtime_wasms[i].paused = true;
                        }
                    }
                }
                else 
                {
                    for _j in 0..runtime_wasms[i].priority
                    {
                        runtime_wasms[i].run_prog();
                    }
                    activity = true;
                }
                if runtime_wasms[i].ended && let Ok(_sended) = msgto_main.send(Messages::Stop(runtime_wasms[i].module.name.clone()))
                {
                    runtime_wasms.remove(i);
                }
                i += 1;
            }
        }
        if !activity
        {
            sleep(time::Duration::from_secs(1));
        }
        i = 0;
    }
}
