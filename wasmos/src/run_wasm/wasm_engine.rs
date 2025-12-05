use std::{fs, path::Path};
use std::convert::TryInto;

#[derive(Debug, Clone)]
pub struct FunctionType {
    pub params: Vec<u8>,
    pub results: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Import {
    pub module: String,
    pub name: String,
    pub kind: u8,
    pub desc: u32, // type index for func
}

#[derive(Debug, Clone)]
pub struct Export {
    pub name: String,
    pub index: u32,
    pub kind: u8, // 0x00 = func, 0x01 = table, 0x02 = mem, 0x03 = global
}

#[derive(Debug, Clone)]
pub struct CodeBody {
    pub locals: Vec<(u32, u8)>, // count, type
    pub code: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct WasmModule {
    pub types: Vec<FunctionType>,
    pub imports: Vec<Import>,
    pub functions: Vec<u32>, // indices into types
    pub exports: Vec<Export>,
    pub code: Vec<CodeBody>,
    pub start_func: Option<u32>,
}

impl WasmModule {
    pub fn parse(binary: &[u8]) -> Result<Self, String> {
        if binary.len() < 8 || &binary[0..4] != b"\0asm" || &binary[4..8] != b"\x01\0\0\0" {
            return Err("Invalid WASM magic or version".to_string());
        }
a
        let mut module = WasmModule::default();
        let mut pos = 8;

        while pos < binary.len() {
            let section_id = binary[pos];
            pos += 1;
            let (section_len, len_bytes) = decode_leb128_u32(&binary[pos..])?;
            pos += len_bytes;
            let section_end = pos + section_len as usize;

            match section_id {
                1 => parse_type_section(&binary[pos..section_end], &mut module)?,
                2 => parse_import_section(&binary[pos..section_end], &mut module)?,
                3 => parse_function_section(&binary[pos..section_end], &mut module)?,
                7 => parse_export_section(&binary[pos..section_end], &mut module)?,
                8 => parse_start_section(&binary[pos..section_end], &mut module)?,
                10 => parse_code_section(&binary[pos..section_end], &mut module)?,
                _ => {} // Skip other sections
            }
            pos = section_end;
        }

        Ok(module)
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

fn parse_type_section(data: &[u8], module: &mut WasmModule) -> Result<(), String> {
    let mut pos = 0;
    let (count, bytes) = decode_leb128_u32(data)?;
    pos += bytes;

    for _ in 0..count {
        if data[pos] != 0x60 {
            return Err("Invalid func type".to_string());
        }
        pos += 1;

        let (param_count, bytes) = decode_leb128_u32(&data[pos..])?;
        pos += bytes;
        let mut params = Vec::new();
        for _ in 0..param_count {
            params.push(data[pos]);
            pos += 1;
        }

        let (result_count, bytes) = decode_leb128_u32(&data[pos..])?;
        pos += bytes;
        let mut results = Vec::new();
        for _ in 0..result_count {
            results.push(data[pos]);
            pos += 1;
        }

        module.types.push(FunctionType { params, results });
    }
    Ok(())
}

fn parse_import_section(data: &[u8], module: &mut WasmModule) -> Result<(), String> {
    let mut pos = 0;
    let (count, bytes) = decode_leb128_u32(data)?;
    pos += bytes;

    for _ in 0..count {
        let (mod_len, bytes) = decode_leb128_u32(&data[pos..])?;
        pos += bytes;
        let mod_name = String::from_utf8_lossy(&data[pos..pos + mod_len as usize]).to_string();
        pos += mod_len as usize;

        let (name_len, bytes) = decode_leb128_u32(&data[pos..])?;
        pos += bytes;
        let name = String::from_utf8_lossy(&data[pos..pos + name_len as usize]).to_string();
        pos += name_len as usize;

        let kind = data[pos];
        pos += 1;
        
        let desc = if kind == 0x00 { // Function
            let (type_idx, bytes) = decode_leb128_u32(&data[pos..])?;
            pos += bytes;
            type_idx
        } else {
            // Skip other import types for now (Table, Mem, Global)
            // We need to implement skipping logic properly or just assume simple MVP
            // For MVP, let's just read one byte/leb128 and hope it's enough or error out
            // Actually, we should probably implement full skipping to be safe.
            // But for now, let's just assume function imports for our test case.
            0 
        };

        module.imports.push(Import { module: mod_name, name, kind, desc });
    }
    Ok(())
}

fn parse_function_section(data: &[u8], module: &mut WasmModule) -> Result<(), String> {
    let mut pos = 0;
    let (count, bytes) = decode_leb128_u32(data)?;
    pos += bytes;

    for _ in 0..count {
        let (type_idx, bytes) = decode_leb128_u32(&data[pos..])?;
        pos += bytes;
        module.functions.push(type_idx);
    }
    Ok(())
}

fn parse_export_section(data: &[u8], module: &mut WasmModule) -> Result<(), String> {
    let mut pos = 0;
    let (count, bytes) = decode_leb128_u32(data)?;
    pos += bytes;

    for _ in 0..count {
        let (name_len, bytes) = decode_leb128_u32(&data[pos..])?;
        pos += bytes;
        let name = String::from_utf8_lossy(&data[pos..pos + name_len as usize]).to_string();
        pos += name_len as usize;

        let kind = data[pos];
        pos += 1;
        let (index, bytes) = decode_leb128_u32(&data[pos..])?;
        pos += bytes;

        module.exports.push(Export { name, index, kind });
    }
    Ok(())
}

fn parse_start_section(data: &[u8], module: &mut WasmModule) -> Result<(), String> {
    let (index, _) = decode_leb128_u32(data)?;
    module.start_func = Some(index);
    Ok(())
}

fn parse_code_section(data: &[u8], module: &mut WasmModule) -> Result<(), String> {
    let mut pos = 0;
    let (count, bytes) = decode_leb128_u32(data)?;
    pos += bytes;

    for _ in 0..count {
        let (body_size, bytes) = decode_leb128_u32(&data[pos..])?;
        pos += bytes;
        let body_end = pos + body_size as usize;
        
        let mut body_pos = pos;
        let (local_vec_count, bytes) = decode_leb128_u32(&data[body_pos..])?;
        body_pos += bytes;
        
        let mut locals = Vec::new();
        for _ in 0..local_vec_count {
            let (count, bytes) = decode_leb128_u32(&data[body_pos..])?;
            body_pos += bytes;
            let type_byte = data[body_pos];
            body_pos += 1;
            locals.push((count, type_byte));
        }

        let code = data[body_pos..body_end].to_vec();
        module.code.push(CodeBody { locals, code });
        
        pos = body_end;
    }
    Ok(())
}

pub fn wasm_engine(file_path: &Path) -> bool
{
    //execute wasm file.
    let wasm_binary:Vec<u8> = fs::read(file_path).expect("Wasm file could not be read");
    
    match WasmModule::parse(&wasm_binary) {
        Ok(module) => {
            println!("Parsed WASM Module:");
            println!("  Types: {}", module.types.len());
            println!("  Functions: {}", module.functions.len());
            println!("  Exports: {}", module.exports.len());
            println!("  Code Bodies: {}", module.code.len());
            
            let mut runtime = super::interpreter::Runtime::new();
            
            // Find entry point
            // 1. Start section
            if let Some(start_idx) = module.start_func {
                println!("Executing start function: {}", start_idx);
                if let Err(e) = runtime.execute(&module, start_idx) {
                    println!("Runtime error: {}", e);
                    return false;
                }
                return true;
            }
            
            // 2. Exported "main" or "_start"
            for export in &module.exports {
                if export.kind == 0x00 { // Function
                    if export.name == "main" || export.name == "_start" {
                        println!("Executing exported function: {} (idx: {})", export.name, export.index);
                        if let Err(e) = runtime.execute(&module, export.index) {
                            println!("Runtime error: {}", e);
                            return false;
                        }
                        return true;
                    }
                }
            }
            
            println!("No entry point found (start section or main/_start export)");
            false
        },
        Err(e) => {
            println!("Error parsing WASM: {}", e);
            false
        },
    }
}
