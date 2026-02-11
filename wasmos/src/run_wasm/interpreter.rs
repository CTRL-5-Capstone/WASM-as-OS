<<<<<<< HEAD
=======
/*use supper::wasm_engine::{WasmModule, CodeBody};
>>>>>>> f9b8ff4d9b5a7677c99e8d80d5febdf9159718d6
use std::collections::HashMap;
use crate::struct_files::wasm_struct::WasmFile;


pub struct Runtime {
    pub stack: Vec<i32>, // Simplified to i32 for now
    pub locals: Vec<i32>,
    pub memory: Vec<u8>,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            locals: Vec::new(),
            memory: vec![0; 65536], // 1 page (64KB)
        }
    }

    pub fn execute(&mut self, module: &WasmFile, func_idx: u32) -> Result<(), String> {
        let num_imports = module.imports.len() as u32;

        // Check if it's an import
        if func_idx < num_imports {
            let import = &module.imports[func_idx as usize];
            if import.module == "env" && import.name == "host_log" {
                // ABI: host_log(ptr: i32, len: i32)
                let len = self.stack.pop().ok_or("Stack underflow")?;
                let ptr = self.stack.pop().ok_or("Stack underflow")?;
                
                // Read string from memory
                let start = ptr as usize;
                let end = start + len as usize;
                if end > self.memory.len() {
                    return Err("Memory access out of bounds".to_string());
                }
                
                let msg = String::from_utf8_lossy(&self.memory[start..end]);
                println!("[WASM LOG] {}", msg);
                return Ok(());
            } else {
                return Err(format!("Unknown import: {}.{}", import.module, import.name));
            }
        }

        // It's a defined function
        let code_idx = func_idx - num_imports;
        if (code_idx as usize) >= module.code.len() {
             return Err(format!("Function index out of bounds: {}", func_idx));
        }
        
        let code_body = &module.code[code_idx as usize];
        
        // Initialize locals
        // Note: In a real implementation, we need to push params from stack to locals.
        // For this MVP, we assume no params for internal functions or handle it simply.
        // Actually, for `main`, there are no params.
        // For called functions, we should pop params.
        // Let's skip param handling for now as our test case is simple.
        
        let mut frame_locals = Vec::new();
        // Copy params from stack? No, for now just initialize defined locals.
        for (count, _type) in &code_body.locals {
            for _ in 0..*count {
                frame_locals.push(0);
            }
        }
        
        // Save previous locals if we were recursive (not handled here, we need a call stack)
        // For simple recursion we need a Frame struct.
        // For this MVP, let's just swap locals and restore them? 
        // Or just use a fresh vector.
        let prev_locals = std::mem::replace(&mut self.locals, frame_locals);

        let mut pc = 0;
        while pc < code_body.code.len() {
            let opcode = code_body.code[pc];
            pc += 1;

            match opcode {
                0x0b => { // end
                    break; 
                }
                0x20 => { // local.get
                    let (idx, len) = decode_leb128_u32(&code_body.code[pc..])?;
                    pc += len;
                    if (idx as usize) < self.locals.len() {
                        self.stack.push(self.locals[idx as usize]);
                    } else {
                        // Check params? We don't have params in locals yet.
                        return Err(format!("local.get index out of bounds: {}", idx));
                    }
                }
                0x21 => { // local.set
                    let (idx, len) = decode_leb128_u32(&code_body.code[pc..])?;
                    pc += len;
                    if let Some(val) = self.stack.pop() {
                        if (idx as usize) < self.locals.len() {
                            self.locals[idx as usize] = val;
                        } else {
                            return Err(format!("local.set index out of bounds: {}", idx));
                        }
                    } else {
                        return Err("Stack underflow".to_string());
                    }
                }
                0x41 => { // i32.const
                    let (val, len) = decode_leb128_i32(&code_body.code[pc..])?;
                    pc += len;
                    self.stack.push(val);
                }
                0x6a => { // i32.add
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    self.stack.push(a.wrapping_add(b));
                }
                0x10 => { // call
                    let (target_idx, len) = decode_leb128_u32(&code_body.code[pc..])?;
                    pc += len;
                    // Recursive call
                    self.execute(module, target_idx)?;
                }
                _ => {
                    // println!("Unknown opcode: 0x{:02x}", opcode);
                }
            }
        }
        
        // Restore locals
        self.locals = prev_locals;
        
        Ok(())
    }
}

fn decode_leb128_u32(slice: &[u8]) -> Result<(u32, usize), String> {
    let mut result: u32 = 0;
    let mut shift = 0;
    let mut count = 0;
    for &byte in slice {
        result |= ((byte & 0x7f) as u32) << shift;
        shift += 7;
        count += 1;
        if byte & 0x80 == 0 {
            return Ok((result, count));
        }
    }
    Err("LEB128 decode failed".to_string())
}

fn decode_leb128_i32(slice: &[u8]) -> Result<(i32, usize), String> {
    let mut result: i32 = 0;
    let mut shift = 0;
    let mut count = 0;
    let mut byte;
    loop {
        if count >= slice.len() {
             return Err("LEB128 decode failed".to_string());
        }
        byte = slice[count];
        result |= ((byte & 0x7f) as i32) << shift;
        shift += 7;
        count += 1;
        if byte & 0x80 == 0 {
            break;
        }
    }
    
    if (shift < 32) && (byte & 0x40 != 0) {
        result |= !0 << shift;
    }
    
    Ok((result, count))
}
*/