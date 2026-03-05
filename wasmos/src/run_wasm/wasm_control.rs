use crate::run_wasm::build_runtime::Runtime;
use dialoguer::{Select, theme::ColorfulTheme};
use crate::struct_files::wasm_list::*;
use std::path::Path;
use std::time;
use std::thread::sleep;
use std::sync::mpsc::TryRecvError;
use std::time::Duration;
use std::sync::mpsc::{Receiver, Sender};

pub enum Messages
{
//    Error(String, bool),
    Flog(String, bool),
    Clog(String, bool),
    Start(Runtime),
    Pause(String),
    Resume(String),
    Stop(String),
    Limit(String, usize, bool),
    Delete(String),
    PrintFlags(),
}
pub enum PrintCode
{
    //enums for all prints
    //I32
    I32Eqz(bool),
    I32Eq(bool),
    I32Ne(bool),
   //flow
    Unreachable(bool),
    Nop(bool),
    Block(bool),
    Loop(bool),
    If(bool),
    Else(bool),
    End(bool),
    Br(bool),
    BrIf(bool),
    BrTable(bool),
    Return(bool),
    Call(bool),
    CallIndirect(bool),
    //Args
    Drop(bool),
    Select(bool),
    //Vars
    LocalGet(bool),
    LocalSet(bool),
    LocalTee(bool),
    GlobalGet(bool),
    GlobalSet(bool),

    //Mem
    //LD
    I32Load(bool),
    I64Load(bool),
    F32Load(bool),
    F64Load(bool),
    //I32
    I32Load8S(bool),
    I32Load8U(bool),
    I32Load16S(bool),
    I32Load16U(bool),
    //I64
    I64Load8S(bool),
    I64Load8U(bool),
    I64Load16S(bool),
    I64Load16U(bool),
    I64Load32S(bool),
    I64Load32U(bool),
    //STR
    I32Store(bool),
    I64Store(bool),
    F32Store(bool),
    F64Store(bool),
    I32Store8(bool),
    I32Store16(bool),
    I64Store8(bool),
    I64Store16(bool),
    I64Store32(bool),
    MemorySize(bool),
    MemoryGrow(bool),
    //Cons
    I32Const(bool),
    I64Const(bool),
    F32Const(bool),
    F64Const(bool),
    //Comps    
    I32LtS(bool),
    I32LtU(bool),
    I32GtS(bool),
    I32GtU(bool),
    I32LeS(bool),
    I32LeU(bool),
    I32GeS(bool),
    I32GeU(bool),
    //I64
    I64Eqz(bool),
    I64Eq(bool),
    I64Ne(bool),
    I64LtS(bool),
    I64LtU(bool),
    I64GtS(bool),
    I64GtU(bool),
    I64LeS(bool),
    I64LeU(bool),
    I64GeS(bool),
    I64GeU(bool),
    //F32
    F32Eq(bool),
    F32Ne(bool),
    F32Lt(bool),
    F32Gt(bool),
    F32Le(bool),
    F32Ge(bool),
    //F64
    F64Eq(bool),
    F64Ne(bool),
    F64Lt(bool),
    F64Gt(bool),
    F64Le(bool),
    F64Ge(bool),
    //Calcs
    //I32
    I32Clz(bool),
    I32Ctz(bool),
    I32Popcnt(bool),
    I32Add(bool),
    I32Sub(bool),
    I32Mul(bool),
    I32DivS(bool),
    I32DivU(bool),
    I32RemS(bool),
    I32RemU(bool),
    I32And(bool),
    I32Or(bool),
    I32Xor(bool),
    I32Shl(bool),
    I32ShrS(bool),
    I32ShrU(bool),
    I32Rotl(bool),
    I32Rotr(bool),
    //I64
    I64Clz(bool),
    I64Ctz(bool),
    I64Popcnt(bool),
    I64Add(bool),
    I64Sub(bool),
    I64Mul(bool),
    I64DivS(bool),
    I64DivU(bool),
    I64RemS(bool),
    I64RemU(bool),
    I64And(bool),
    I64Or(bool),
    I64Xor(bool),
    I64Shl(bool),
    I64ShrS(bool),
    I64ShrU(bool),
    I64Rotl(bool),
    I64Rotr(bool),
    //FL
    //F32
    F32Abs(bool),
    F32Neg(bool),
    F32Ceil(bool),
    F32Floor(bool),
    F32Trunc(bool),
    F32Nearest(bool),
    F32Sqrt(bool),
    F32Add(bool),
    F32Sub(bool),
    F32Mul(bool),
    F32Div(bool),
    F32Min(bool),
    F32Max(bool),
    F32Copysign(bool),
    //F64
    F64Abs(bool),
    F64Neg(bool),
    F64Ceil(bool),
    F64Floor(bool),
    F64Trunc(bool),
    F64Nearest(bool),
    F64Sqrt(bool),
    F64Add(bool),
    F64Sub(bool),
    F64Mul(bool),
    F64Div(bool),
    F64Min(bool),
    F64Max(bool),
    F64Copysign(bool),
    //tools
    I32WrapI64(bool),
    I32TruncF32S(bool),
    I32TruncF32U(bool),
    I32TruncF64S(bool),
    I32TruncF64U(bool),
    I64ExtendI32S(bool),
    I64ExtendI32U(bool),
    I64TruncF32S(bool),
    I64TruncF32U(bool),
    I64TruncF64S(bool),
    I64TruncF64U(bool),
    F32ConvertI32S(bool),
    F32ConvertI32U(bool),
    F32ConvertI64S(bool),
    F32ConvertI64U(bool),
    F32DemoteF64(bool),
    F64ConvertI32S(bool),
    F64ConvertI32U(bool),
    F64ConvertI64S(bool),
    F64ConvertI64U(bool),
    F64PromoteF32(bool),
    I32ReinterpretF32(bool),
    I64ReinterpretF64(bool),
    F32ReinterpretI32(bool),
    F64ReinterpretI64(bool),
}
pub fn update_from_thread(shared_list: &mut WasmList, from_thread: &mut Receiver<Messages>)
{
        let wasm_tup = shared_list.list_runningvec();
        let running_list = wasm_tup.1;
        let wasm_vec = wasm_tup.0;
        loop
        {
            match from_thread.try_recv()
            {
                Ok(mesg) => {
                    match mesg
                    {
                        Messages::Stop(wasm_name) =>
                        {
                            if let Ok(i) = running_list.binary_search(&wasm_name)
                            {
                                wasm_vec[i].lock().unwrap().wasm_file.run_false();
                            }
                        }
                        _ => panic!("Impossible Message from Thread"),
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => panic!("Critical Error Thread Unresponsive"),
            }
            sleep(time::Duration::from_millis(100));
        }  
}
pub fn start_wasm_by_id(wasm_list: &mut WasmList, id: &str) {
    let wasm_tup = wasm_list.list_haltedvec();
    let wasm_vec = wasm_tup.0;
    
    for wasm in wasm_vec {
        if  wasm.lock().unwrap().wasm_file.name == id {
            let path_name = wasm.lock().unwrap().wasm_file.path_to.clone();
            let _path = Path::new(&path_name);
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
{//Function provides a menu for assigning a new runtime settings.
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
    //Array for limit menu
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
            1 => { //Case For setting a instruction limit.
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
            2 => { //Case for console logging
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
            3 => { //Case for file logging
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
            4 => { //Case for starting with the runtime paused
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
{//Function allows user to change settings for a runtime.
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
    //Array for limit menu
    let larr = [
        "Custom",
        "500",
        "1000",
        "2500",
        "5000",
    ];
    let mut menu_arr:[&str; 4] = [""; 4];
    //Populate menu with currents
    menu_arr[0] = false_arr[0];
    if runtime.limflag {menu_arr[1] = true_arr[1];}
    else{menu_arr[1] = false_arr[1];}
    if runtime.clog {menu_arr[2] = true_arr[2];}
    else{menu_arr[2] = false_arr[2]};
    if runtime.flog {menu_arr[3] = true_arr[3];}
    else{menu_arr[3] = false_arr[3];}


    let mut choice = 0;
    loop{
        choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Runtime Options")
        .items(&menu_arr)
        .default(choice)
        .interact()
        .unwrap();
        match choice{
            0 => break, //Return Case
            1 => { //Case for enabling/disableing 
                if !runtime.limflag
                {
                    let lchoice = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Runtime Options")
                    .items(&larr)
                    .default(0)
                    .interact()
                    .unwrap();
                    match lchoice{
                        0 => { //Case for custom limit prompts for limit 
                            let mut slimit = String::new();
                            std::io::stdin().read_line(&mut slimit).expect("Limit Input Error Runtime Options"); //Will add graceful error handling
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

#[cfg(test)]
mod tests {

    #[test]
    fn test_boolean_true() {
        let flag = true;
        assert!(flag);
    }

    #[test]
    fn test_boolean_false() {
        let flag = false;
        assert!(!flag);
    }

    #[test]
    fn test_flag_toggle() {
        let mut flag = false;

        flag = !flag;

        assert!(flag);
    }

    #[test]
    fn test_integer_comparison() {
        let a = 5;
        let b = 5;

        assert_eq!(a, b);
    }

    #[test]
    fn test_integer_addition() {
        let result = 2 + 3;

        assert_eq!(result, 5);
    }
}
