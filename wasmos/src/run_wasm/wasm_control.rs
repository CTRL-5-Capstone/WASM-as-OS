use crate::run_wasm::build_runtime::{Runtime, PFlags};
use dialoguer::{Select, theme::ColorfulTheme};
use crate::struct_files::wasm_list::*;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::thread::sleep;
use std::sync::mpsc::TryRecvError;
use std::sync::mpsc::{Receiver, Sender};

pub enum RuntimeMessages
{
    Start(Runtime),
    ScheduleRuntime(u64, Runtime),
}
pub enum Messages
{
//    Error(String, bool),
    Flog(String, bool),
    Clog(String, bool),
    Pause(String),
    Resume(String),
    Stop(String),
    Limit(String, usize, bool),
    Delete(String),
    PrintFlags(String, bool, Vec<PrintCode>),
    AllFlags(String, bool, bool)
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
            sleep(Duration::from_millis(100));
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
pub fn new_runtime_options(mut runtime: Runtime, to_thread: Sender<RuntimeMessages>)
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
                to_thread.send(RuntimeMessages::Start(runtime)).expect("Critical Error Thread Unresponsive");
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
pub fn start_wasm(wasm_list: &mut WasmList, to_thread: Sender<RuntimeMessages>)
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
pub fn runtime_loop(msgto_main: Sender<Messages>, msgfrom_main: Receiver<Messages>, rmsgfrm_main: Receiver<RuntimeMessages>)
{
    let mut activity;
    let mut runtime_wasms: Vec<Runtime> = Vec::new();
    let mut scheduled_wasms: Vec<(u64, Runtime)> = Vec::new();
    let mut i = 0;
    loop
    {
        while i < scheduled_wasms.len()
        {
            let cur_time = match SystemTime::now().duration_since(UNIX_EPOCH)
            {
                Ok(time) => time.as_secs(),
                _ => panic!("System Clock is Incorrect"),

            };
            if scheduled_wasms[i].0 <= cur_time
            {
                let runt = scheduled_wasms[i].1.clone();
                scheduled_wasms.remove(i);
                let mut j = 0;
                while j < runtime_wasms.len()
                {
                    if runtime_wasms[j].module.name < scheduled_wasms[j].1.module.name{break;}
                    j += 1;
                }
                runtime_wasms.insert(j, runt);     
            }
            else{i += 1;}
        }
        while let Ok(mssg) = rmsgfrm_main.try_recv()
        {
            match mssg
            {
                RuntimeMessages::Start(rtime) => {
                    while i < runtime_wasms.len()
                    {
                        if runtime_wasms[i].module.name < rtime.module.name{break;}
                    }
                    runtime_wasms.insert(i, rtime);
                }
                RuntimeMessages::ScheduleRuntime(tim, runt) => scheduled_wasms.push((tim, runt)), 
            }
            sleep(Duration::from_millis(1));
            i = 0;
        }
        while let Ok(mssg) = msgfrom_main.try_recv()
        {
            match mssg{
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
                Messages::PrintFlags(name, cftyp, flags) => {
                    if let Ok(ind) = runtime_wasms.binary_search_by_key(&name, |rtime| rtime.module.name.clone())
                    {
                        for flag in flags{
                            match flag{
                                //enums for all prints
                                //I32
                                PrintCode::I32Eqz(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32Eq(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32Ne(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                //flow
                                PrintCode::Unreachable(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::Nop(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::Block(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::Loop(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::If(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::Else(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::End(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::Br(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::BrIf(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::BrTable(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::Return(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::Call(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::CallIndirect(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                //Args
                                PrintCode::Drop(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::Select(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                //Vars
                                PrintCode::LocalGet(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::LocalSet(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::LocalTee(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::GlobalGet(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::GlobalSet(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }

                                //Mem
                                //LD
                                PrintCode::I32Load(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64Load(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32Load(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64Load(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                //I32
                                PrintCode::I32Load8S(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32Load8U(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32Load16S(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32Load16U(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                //I64
                                PrintCode::I64Load8S(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64Load8U(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64Load16S(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64Load16U(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64Load32S(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64Load32U(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                //STR
                                PrintCode::I32Store(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64Store(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32Store(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64Store(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32Store8(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32Store16(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64Store8(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64Store16(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64Store32(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::MemorySize(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::MemoryGrow(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                //Cons
                                PrintCode::I32Const(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64Const(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32Const(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64Const(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                //Comps    
                                PrintCode::I32LtS(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32LtU(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32GtS(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32GtU(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32LeS(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32LeU(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32GeS(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32GeU(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                //I64
                                PrintCode::I64Eqz(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64Eq(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64Ne(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64LtS(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64LtU(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64GtS(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64GtU(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64LeS(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64LeU(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64GeS(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64GeU(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                //F32
                                PrintCode::F32Eq(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32Ne(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32Lt(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32Gt(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32Le(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32Ge(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                //F64
                                PrintCode::F64Eq(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64Ne(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64Lt(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64Gt(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64Le(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64Ge(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                //Calcs
                                //I32
                                PrintCode::I32Clz(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32Ctz(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32Popcnt(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32Add(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32Sub(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32Mul(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32DivS(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32DivU(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32RemS(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32RemU(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32And(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32Or(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32Xor(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32Shl(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32ShrS(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32ShrU(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32Rotl(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32Rotr(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                //I64
                                PrintCode::I64Clz(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64Ctz(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64Popcnt(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64Add(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64Sub(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64Mul(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64DivS(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64DivU(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64RemS(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64RemU(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64And(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64Or(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64Xor(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64Shl(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64ShrS(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64ShrU(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64Rotl(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64Rotr(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                //FL
                                //F32
                                PrintCode::F32Abs(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32Neg(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32Ceil(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32Floor(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32Trunc(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32Nearest(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32Sqrt(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32Add(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32Sub(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32Mul(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32Div(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32Min(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32Max(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32Copysign(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                //F64
                                PrintCode::F64Abs(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64Neg(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64Ceil(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64Floor(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64Trunc(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64Nearest(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64Sqrt(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64Add(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64Sub(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64Mul(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64Div(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64Min(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64Max(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64Copysign(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                //tools
                                PrintCode::I32WrapI64(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32TruncF32S(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32TruncF32U(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32TruncF64S(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32TruncF64U(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64ExtendI32S(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64ExtendI32U(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64TruncF32S(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64TruncF32U(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64TruncF64S(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64TruncF64U(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32ConvertI32S(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32ConvertI32U(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32ConvertI64S(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32ConvertI64U(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32DemoteF64(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64ConvertI32S(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64ConvertI32U(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64ConvertI64S(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64ConvertI64U(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64PromoteF32(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I32ReinterpretF32(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::I64ReinterpretF64(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F32ReinterpretI32(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }
                                PrintCode::F64ReinterpretI64(flag) => {
                                    if cftyp{runtime_wasms[ind].spflags.i32_eqz = flag;}
                                    else{runtime_wasms[ind].fpflags.i32_eqz = flag;}
                                }    
                            }
                        }
                    }
                    else{
                       println!("Runtime: {} has ended.", name);
                        continue;
                    }
                }
                _ => (),
            }
            sleep(Duration::from_millis(1));
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
                        else
                        {
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
            sleep(Duration::from_secs(1));
        }
        i = 0;
    }
}
