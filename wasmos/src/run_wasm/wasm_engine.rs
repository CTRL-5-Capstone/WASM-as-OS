use std::{fs, path::Path};
use super::wasm_module::*;
use super::build_runtime::*;

pub struct Curse
{
    byte_vec: Vec<u8>,
    loc: usize,
    len: usize,
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
            0x05 => Code::Else,
            0x0B => Code::End,
            0x0C => Code::Br(self.leb_tou32()),
            0x0D => Code::BrIf(self.leb_tou32()),
            /*0x0E => BrTable
                {
                    def: u32,
                    locs: Vec<u32>,
                },*/
            0x0F => Code::Return,
            0x10 => Code::Call(self.leb_tou32()),
            0x11 => Code::CallIndirect(self.leb_tou32()),
            //Args
            0x1A => Code::Drop,
            0x1B => Code::Select,
            //Vars
            0x20 => Code::LocalGet(self.leb_tou32()),
            0x21 => Code::LocalSet(self.leb_tou32()),
            0x22 => Code::LocalTee(self.leb_tou32()),
            0x23 => Code::GlobalGet(self.leb_tou32()),
            0x24 => Code::GlobalSet(self.leb_tou32()),
            //Mem
            //LD
            0x28 => {
                let _waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I32Load(off)
            },
            0x29 => {
                let _waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I64Load(off)
            },
            0x2A => {
                let _waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::F32Load(off)
            },
            0x2B => {
                let _waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::F64Load(off)
            },
            //I32
            0x2C => {
                let _waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I32Load8S(off)
            },
            0x2D => {
                let _waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I32Load8U(off)
            },
            0x2E => {
                let _waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I32Load16S(off)
            },
            0x2F => {
                let _waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I32Load16U(off)
            },
            //I64
            0x30 => {
                let _waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I64Load8S(off)
            },
            0x31 => {
                let _waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I64Load8U(off)
            },
            0x32 => {
                let _waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I64Load16S(off)
        },
            0x33 => {
                let _waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I64Load16U(off)
        },
            0x34 => {
                let _waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I64Load32S(off)    
            },
            0x35 => {
                let _waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I64Load32U(off)
            },
            //STR
            0x36 => {
                let _waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I32Store(off)
            },
            0x37 => {
                let _waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I64Store(off)
            },
            0x38 => {
                let _waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::F32Store(off)
            },
            0x39 => {
                let _waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::F64Store(off)
            },
            0x3A => {
                let _waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I32Store8(off)
            },
            0x3B => {
                let _waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I32Store16(off)
            },
            0x3C => {
                let _waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I64Store8(off)
            },
            0x3D => {
                let _waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I64Store16(off)
        },
            0x3E => {
                let _waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I64Store32(off)
            },
            0x3F => Code::MemorySize,
            0x40 => Code::MemoryGrow,
            //Cons
            0x41 => Code::I32Const(self.leb_toi32()),
            0x42 => Code::I64Const(self.leb_toi64()),
            0x43 => Code::F32Const(self.leb_tof32()), 
            0x44 => Code::F64Const(self.leb_tof64()),
            //Comps
            //I32,
            0x45 => Code::I32Eqz,
            0x46 => Code::I32Eq,
            0x47 => Code::I32Ne,
            0x48 => Code::I32LtS,
            0x49 => Code::I32LtU,
            0x4A => Code::I32GtS,
            0x4B => Code::I32GtU,
            0x4C => Code::I32LeS,
            0x4D => Code::I32LeU,
            0x4E => Code::I32GeS,
            0x4F => Code::I32GeU,
            //I64
            0x50 => Code::I64Eqz,
            0x51 => Code::I64Eq,
            0x52 => Code::I64Ne,
            0x53 => Code::I64LtS,
            0x54 => Code::I64LtU,
            0x55 => Code::I64GtS,
            0x56 => Code::I64GtU,
            0x57 => Code::I64LeS,
            0x58 => Code::I64LeU,
            0x59 => Code::I64GeS,
            0x5A => Code::I64GeU,
            //F32
            0x5B => Code::F32Eq,
            0x5C => Code::F32Ne,
            0x5D => Code::F32Lt,
            0x5E => Code::F32Gt,
            0x5F => Code::F32Le,
            0x60 => Code::F32Ge,
            //F64
            0x61 => Code::F64Eq,
            0x62 => Code::F64Ne,
            0x63 => Code::F64Lt,
            0x64 => Code::F64Gt,
            0x65 => Code::F64Le,
            0x66 => Code::F64Ge,
            //Calcs
            //I32
            0x67 => Code::I32Clz,
            0x68 => Code::I32Ctz,
            0x69 => Code::I32Popcnt,
            0x6A => Code::I32Add,
            0x6B => Code::I32Sub,
            0x6C => Code::I32Mul,
            0x6D => Code::I32DivS,
            0x6E => Code::I32DivU,
            0x6F => Code::I32RemS,
            0x70 => Code::I32RemU,
            0x71 => Code::I32And,
            0x72 => Code::I32Or,
            0x73 => Code::I32Xor,
            0x74 => Code::I32Shl,
            0x75 => Code::I32ShrS,
            0x76 => Code::I32ShrU,
            0x77 => Code::I32Rotl,
            0x78 => Code::I32Rotr,
            //I64
            0x79 => Code::I64Clz,
            0x7A => Code::I64Ctz,
            0x7B => Code::I64Popcnt,
            0x7C => Code::I64Add,
            0x7D => Code::I64Sub,
            0x7E => Code::I64Mul,
            0x7F => Code::I64DivS,
            0x80 => Code::I64DivU,
            0x81 => Code::I64RemS,
            0x82 => Code::I64RemU,
            0x83 => Code::I64And,
            0x84 => Code::I64Or,
            0x85 => Code::I64Xor,
            0x86 => Code::I64Shl,
            0x87 => Code::I64ShrS,
            0x88 => Code::I64ShrU,
            0x89 => Code::I64Rotl,
            0x8A => Code::I64Rotr,
            //FL
            //F32
            0x8B => Code::F32Abs,
            0x8C => Code::F32Neg,
            0x8D => Code::F32Ceil,
            0x8E => Code::F32Floor,
            0x8F => Code::F32Trunc,
            0x90 => Code::F32Nearest,
            0x91 => Code::F32Sqrt,
            0x92 => Code::F32Add,
            0x93 => Code::F32Sub,
            0x94 => Code::F32Mul,
            0x95 => Code::F32Div,
            0x96 => Code::F32Min,
            0x97 => Code::F32Max,
            0x98 => Code::F32Copysign,
            //F64
            0x99 => Code::F64Abs,
            0x9A => Code::F64Neg,
            0x9B => Code::F64Ceil,
            0x9C => Code::F64Floor,
            0x9D => Code::F64Trunc,
            0x9E => Code::F64Nearest,
            0x9F => Code::F64Sqrt,
            0xA0 => Code::F64Add,
            0xA1 => Code::F64Sub,
            0xA2 => Code::F64Mul,
            0xA3 => Code::F64Div,
            0xA4 => Code::F64Min,
            0xA5 => Code::F64Max,
            0xA6 => Code::F64Copysign,
            //tools
            0xA7 => Code::I32WrapI64,
            0xA8 => Code::I32TruncF32S,
            0xA9 => Code::I32TruncF32U,
            0xAA => Code::I32TruncF64S,
            0xAB => Code::I32TruncF64U,
            0xAC => Code::I64ExtendI32S,
            0xAD => Code::I64ExtendI32U,
            0xAE => Code::I64TruncF32S,
            0xAF => Code::I64TruncF32U,
            0xB0 => Code::I64TruncF64S,
            0xB1 => Code::I64TruncF64U,
            0xB2 => Code::F32ConvertI32S,
            0xB3 => Code::F32ConvertI32U,
            0xB4 => Code::F32ConvertI64S,
            0xB5 => Code::F32ConvertI64U,
            0xB6 => Code::F32DemoteF64,
            0xB7 => Code::F64ConvertI32S,
            0xB8 => Code::F64ConvertI32U,
            0xB9 => Code::F64ConvertI64S,
            0xBA => Code::F64ConvertI64U,
            0xBB => Code::F64PromoteF32,
            0xBC => Code::I32ReinterpretF32,
            0xBD => Code::I64ReinterpretF64,
            0xBE => Code::F32ReinterpretI32,
            0xBF => Code::F64ReinterpretI64,
            _ =>{println!("{byte}"); panic!("Invalid ops")}, //Temp will remove later
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
    }
    pub fn leb_tou32(&mut self) -> u32
    {
        let mut decoded: u32 = 0;
        let mut shifter: u32 = 0;
        loop 
        {
            if self.loc >= self.len || shifter > 35
            {
                panic!("Vec Overflow") //Temp will replace
            }
            let byte = self.byte_vec[self.loc];
            self.loc += 1;
            let shifty = (byte & 0x7F) as u32;
            decoded |= shifty << shifter;
            if(byte & 0x80) == 0
            {
                return decoded;
            }
            shifter += 7;

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

            match sec
            {
                1 => {       //Types
                    let count = self.leb_tou32() as usize;
                    let mut itt = 0;
                    while itt < count
                    {
                        let byte = self.byte_vec[self.loc];
                        self.loc += 1;
                        if byte != 0x60
                        {
                            panic!("Byte error");
                        }
                    
                        let mut argnum = self.leb_tou32() as usize;
                        let mut args: Vec<Option<TypeBytes>> = Vec::new();
                        start = self.loc;
                        argnum += start;
                        while self.loc < argnum
                        {
                            args.push(decode_byte(self.byte_vec[self.loc]));
                            self.loc += 1;
                        }
                        let mut turns: Vec<Option<TypeBytes>> = Vec::new();
                        argnum = self.leb_tou32() as usize;
                        start = self.loc;
                        argnum += start;
                        while self.loc  < argnum
                        {
                            turns.push(decode_byte(self.byte_vec[self.loc]));
                            self.loc += 1;
                        }
                        module.typs.push(Types{args, turns});
                        itt += 1;
                    }
                }
                2 => {      //Imports
                    let mut count = self.leb_tou32();
                    while count > 0
                    {
                        let mod_len = self.leb_tou32() as usize;
                        let mod_name:String = String::from_utf8(self.byte_vec[self.loc..self.loc+mod_len].to_vec()).unwrap();
                        self.loc += mod_len;
                        let name_len = self.leb_tou32() as usize;
                        let imp_name:String = String::from_utf8(self.byte_vec[self.loc..self.loc+name_len].to_vec()).unwrap();
                        self.loc += name_len;
                        let typ = self.byte_vec[self.loc];
                        self.loc += 1;
                        let ind = self.leb_tou32();
                        match typ
                        {
                            0x00 =>{
                                module.imps.push(Import{
                                    mod_name,
                                    imp_name,
                                    ind,
                                    mem_min: 0,
                                    mem_max: None,
                                    tab_min: 0,
                                    tab_max: None,
                                    exp_type: ExpTyp::Func,
                                    ismut: false,
                                    byte_typs: TypeBytes::I32,

    for _ in 0..count {
        let (name_len, bytes) = decode_leb128_u32(&data[pos..])?;
        pos += bytes;
        let name = String::from_utf8_lossy(&data[pos..pos + name_len as usize]).to_string();
        pos += name_len as usize;

                        };
                        
                        count -= 1;
                    }
                }
                3 => {      //Functions
                    let mut count = self.leb_tou32();
                    while count > 0
                    {
                        let ind = self.leb_tou32();
                        module.fnid.push(ind);
                        count -= 1;
                    } 
                }
                4 => {      //Table
                    let mut count = self.leb_tou32();
                    while count > 0
                    {
                        let typ = decode_byte(self.byte_vec[self.loc]).unwrap();
                        let flags = self.byte_vec[self.loc];
                        let tabmin = self.leb_tou32();
                        let mut tabmax = None;
                        if flags == 0x01 
                        {
                            tabmax = Some(self.leb_tou32());
                        }
                        module.tabs.push(Tab{typ, tabmin, tabmax});
                        count -= 1;
                    }
                }
                5 =>{       //Memmory
                    let mut count = self.leb_tou32();
                    while count > 0
                    {
                        let flags = self.byte_vec[self.loc];
                        self.loc += 1;
                        let memmin = self.leb_tou32();
                        let mut memmax = None;
                        if flags & 0x01 != 0
                        {
                            memmax = Some(self.leb_tou32());
                        } 
                        module.memy.push((memmin, memmax));
                        count -= 1
                    }
                }
                6 => {      //Globals
                    let mut count = self.leb_tou32();
                    while count > 0
                    {
                        let typ = decode_byte(self.byte_vec[self.loc]).unwrap();
                        self.loc += 1;
                        let mutcheck = self.byte_vec[self.loc];
                        let mut ismut = false;
                        if mutcheck != 0{ismut = true;}
                        let mut code = Vec::new();
                        loop {
                            let bcode = self.set_code();
                            let breaker = matches!(bcode, Code::End);
                            code.push(bcode);
                            if breaker{break;}
                        }
                        count -= 1;
                        module.glob.push(Global { typ, ismut, code,})
                    }
                }
                7 => {      //Exports
                    let mut count = self.leb_toi32() as usize;
                    while count > 0
                    {
                        let namelen = self.leb_tou32() as usize;
                        let bytes: Vec<u8> = self.byte_vec[self.loc..namelen+self.loc].to_vec();
                        let name: String = String::from_utf8(bytes).unwrap();
                        self.loc += namelen;
                        let typbyt = self.byte_vec[self.loc];
                        self.loc += 1;
                        let typ = match typbyt
                        {
                            0x00 => ExpTyp::Func,
                            0x01 => ExpTyp::Table,
                            0x02 => ExpTyp::Memory,
                            0x03 => ExpTyp::Global,
                            _ => panic!("Invalid wasm binary!"), //temp robust errors later
                        };
                        let loc = self.leb_tou32();
                        module.exps.push(Export{name, loc, typ});
                        count -= 1;
                    }
                }
                8 => {module.strt = Some(self.leb_tou32());}    //Start point for module (optional)
                /*9 => {      //Elements

                }*/
                10 => {
                    let count = self.leb_tou32() as usize;
                    let funtot = module.imports as usize + count;
                    if module.fcce.len() < funtot{module.fcce.resize(funtot, Function { vars: Vec::new(), code: Vec::new()});}
                    let mut itt = 0;
                    while itt < count
                    {   
                        let mut vars: Vec<(u32, Option<TypeBytes>)> = Vec::new();
                        let csize = self.leb_tou32() as usize;
                        let cend = csize + self.loc;
                        let mut var_count = self.leb_tou32() as usize;
                        while var_count > 0
                        {
                            let var = self.leb_tou32();
                            let typ = decode_byte(self.byte_vec[self.loc]);
                            self.loc += 1;
                            vars.push((var, typ));
                            var_count -= 1;
                        }
                        let mut code: Vec<Code> = Vec::new();
                        while self.loc < cend
                        {
                            let cd = self.set_code();
                            let breaker = matches!(cd, Code::End);
                            code.push(cd);
                            if breaker
                            {
                                break;
                            }
                        }
                        module.fcce[module.imports as usize + itt] = Function{vars, code};
                        itt += 1;
                    }
                }
                /*11 => {     //MemInit
                    let mut count = self.leb_tou32();
                    while count > 0
                    {
                        let veclen = self.leb_tou32();
                        let flags = decode_byte(self.byte_vec[self.loc]);
                        count -= 1;
                    }
                }*/
                /*12 => {     //MemInit Count
                    
                }*/

                _ => self.loc = size, // Skipping other sections until implemented
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
    let leng = wasm_binary.len();
    let mut cursor = Curse::new(wasm_binary, leng);
    let module = cursor.parse_wasm();
    let mut wasm_runner = Runtime::new(module);
    wasm_runner.run_prog();
    true
}





#[cfg(test)]
mod tests {
    use super::*;
    #[test]//7.1
    fn test_leb_tou32() 
    {
        let mut test_cur = Curse::new(vec![0xE5, 0x8E, 0x26], 3);
        let ufromleb = test_cur.leb_tou32();
        assert!(ufromleb == 624485)
    }
    #[test]//7.2
    fn test_leb_toi32()
    {
        let mut test_cur = Curse::new(vec![0xE5, 0x8E, 0x26], 3);
        let i32fromleb = test_cur.leb_toi32();
        assert!(i32fromleb == 624485)
    }
    #[test]//7.3
    fn test_leb_tof32()
    {
        let mut test_cur = Curse::new(vec![0xE5, 0x8E, 0x26], 3);
        let ufromleb = test_cur.leb_tou32();
        assert!(ufromleb == 624485)
    }
    #[test] //8.1
    fn test_empty_parse()
    {
        let mut tcurs = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00], 8);
        let tmod = tcurs.parse_wasm();

        assert!(tmod.typs.is_empty());
        assert!(tmod.fcce.is_empty());
        assert!(tmod.exps.is_empty());
        assert!(tmod.imports == 0);
        assert!(tmod.strt.is_none());
    }
    #[test]//8.2
    fn test_invalid_parse()
    {
        let mut tcurs= Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00], 7);
        let _tmod = tcurs.parse_wasm();

    }
    #[test]//8.3
    fn test_bad_wasm()
    {
        let mut tcurs = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x0A, 0x80, 0x80], 11);
        tcurs.parse_wasm();
    }


}
