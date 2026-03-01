use crate::run_wasm::build_runtime::Runtime;
use std::time;
use std::thread::sleep;
use std::time::Duration;
use std::sync::mpsc::{Receiver, Sender};

/// Messages used for communication between the main thread and the runtime worker thread.
#[derive(Clone)]
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

/// Execute a WASM file by path, returning a rich ExecutionResult.
/// Parses the WASM, initializes a Runtime, runs to completion, and collects metrics.
pub fn execute_wasm_file(path_str: &str) -> Result<super::execution_result::ExecutionResult, String> {
    let path = std::path::Path::new(path_str);
    if !path.exists() {
        return Err(format!("WASM file not found: {}", path_str));
    }

    let file_name = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut runtime = super::wasm_engine::wasm_engine(file_name, path);
    let start = std::time::Instant::now();

    // Initialize entry point
    runtime.pop_run();

    // Run to completion
    while !runtime.ended {
        runtime.run_prog();
    }

    let duration_us = start.elapsed().as_micros() as u64;
    let memory_used = runtime.mem.len() as u64;

    // Build return value string from value stack
    let return_value = if !runtime.value_stack.is_empty() {
        Some(format!("{:?}", runtime.value_stack.last().unwrap()))
    } else {
        None
    };

    Ok(super::execution_result::ExecutionResult::success(
        runtime.instruction_count,
        runtime.syscall_count,
        memory_used,
        duration_us,
        runtime.stdout_log,
        return_value,
    ))
}

/// The background runtime loop that drives step-based execution of multiple WASM modules.
/// Receives control messages from the main thread and executes WASM instructions
/// according to each runtime's priority and limit settings.
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
                if runtime_wasms[i].ended {
                    if let Ok(_sended) = msgto_main.send(Messages::Stop(runtime_wasms[i].module.name.clone()))
                    {
                        runtime_wasms.remove(i);
                    }
                } else {
                    i += 1;
                }
            } else {
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
