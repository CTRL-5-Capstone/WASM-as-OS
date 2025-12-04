use std::{fs, path::Path};
use super::wasm_module::*;
use super::build_runtime::*;
pub struct Curse
{
    byte_vec: Vec<u8>,
    loc: usize,
    len: usize,
}
impl Curse
{
    pub fn set_code(&mut self) -> Code
    { 
        if self.loc >= self.len
        {
            panic!("Cursor Bounds Breached!!!");
        }
        let byte = self.byte_vec[self.loc];
        self.loc += 1;
        match byte
        {    
            //flow
            0x00 => Code::Unreachable,
            0x01 => Code::Nop,
            0x02 => {
                let typ = decode_byte(self.byte_vec[self.loc]);
                self.loc += 1;
                Code::Block(typ)
            },
            0x03 =>{ 
                
                let typ = decode_byte(self.byte_vec[self.loc]);
                self.loc += 1;
                Code::Loop(typ)
            }
            0x04 =>{
                let typ = decode_byte(self.byte_vec[self.loc]);
                self.loc += 1;   
                Code::If(typ)
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
                let waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I32Load(off)
            },
            0x29 => {
                let waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I64Load(off)
            },
            0x2A => {
                let waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::F32Load(off)
            },
            0x2B => {
                let waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::F64Load(off)
            },
            //I32
            0x2C => {
                let waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I32Load8S(off)
            },
            0x2D => {
                let waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I32Load8U(off)
            },
            0x2E => {
                let waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I32Load16S(off)
            },
            0x2F => {
                let waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I32Load16U(off)
            },
            //I64
            0x30 => {
                let waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I64Load8S(off)
            },
            0x31 => {
                let waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I64Load8U(off)
            },
            0x32 => {
                let waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I64Load16S(off)
        },
            0x33 => {
                let waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I64Load16U(off)
        },
            0x34 => {
                let waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I64Load32S(off)    
            },
            0x35 => {
                let waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I64Load32U(off)
            },
            //STR
            0x36 => {
                let waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I32Store(off)
            },
            0x37 => {
                let waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I64Store(off)
            },
            0x38 => {
                let waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::F32Store(off)
            },
            0x39 => {
                let waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::F64Store(off)
            },
            0x3A => {
                let waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I32Store8(off)
            },
            0x3B => {
                let waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I32Store16(off)
            },
            0x3C => {
                let waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I64Store8(off)
            },
            0x3D => {
                let waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I64Store16(off)
        },
            0x3E => {
                let waste = self.leb_tou32();
                let off = self.leb_tou32();
                Code::I64Store32(off)
            },
            0x3F => Code::MemorySize,
            0x40 => Code::MemoryGrow,
            //Cons
            0x41 => Code::I32Const(self.leb_toi32()),
            0x42 => Code::I64Const(self.leb_toi64()),
            //0x43 => Code::F32Const(self.leb_tof32()), func not made yet
            //0x44 => Code::F64Const(self.leb_tof64()),
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
            //F32Ge,
            //F64
            //F64Eq,
            //F64Ne,
            //F64Lt,
            //F64Gt,
            //F64Le,
            //F64Ge,
            //Calcs
            //I32
            //I32Clz,
            //I32Ctz,
            //I32Popcnt,
            0x6A => Code::I32Add,
            0x6B => Code::I32Sub,
            0x6C => Code::I32Mul,
            0x6D => Code::I32DivS,
            0x6E => Code::I32DivU,
            //I32RemS,
            //I32RemU,
            //I32And,
            //I32Or,
            //I32Xor,
            //I32Shl,
            //I32ShrS,
            //I32ShrU,
            //I32Rotl,
            //I32Rotr,
            //I64
            //I64Clz,
            //I64Ctz,
            //I64Popcnt,
            //I64Add,
            //I64Sub,
            //I64Mul,
            //I64DivS,
            //I64DivU,
            //I64RemS,
            //I64RemU,
            //I64And,
            //I64Or,
            //I64Xor,
            //I64Shl,
            //I64ShrS,
            //I64ShrU,
            //I64Rotl,
            //I64Rotr,
            //FL
            //F32
            //F32Abs,
            //F32Neg,
            //F32Ceil,
            //F32Floor,
            //F32Trunc,
            //F32Nearest,
            //F32Sqrt,
            //F32Add,
            //F32Sub,
            //F32Mul,
            //F32Div,
            //F32Min,
            //F32Max,
            //F32Copysign,
            //F64
            //F64Abs,
            //F64Neg,
            //F64Ceil,
            //F64Floor,
            //F64Trunc,
            //F64Nearest,
            //F64Sqrt,
            //F64Add,
            //F64Sub,
            //F64Mul,
            //F64Div,
            //F64Min,
            //F64Max,
            //F64Copysign,
            //tools
            //I32WrapI64,
            //I32TruncF32S,
            //I32TruncF32U,
            //I32TruncF64S,
            //I32TruncF64U,
            //I64ExtendI32S,
            //I64ExtendI32U,
            //I64TruncF32S,
            //I64TruncF32U,
            //I64TruncF64S,
            //I64TruncF64U,
            //F32ConvertI32S,
            //F32ConvertI32U,
            //F32ConvertI64S,
            //F32ConvertI64U,
            //F32DemoteF64,
            //F64ConvertI32S,
            //F64ConvertI32U,
            //F64ConvertI64S,
            //F64ConvertI64U,
            //F64PromoteF32,
            //I32ReinterpretF32,
            //I64ReinterpretF64,
            //F32ReinterpretI32,
            //F64ReinterpretI64,
            _ => panic!("Invalid ops") //Temp will remove later
        }
    }
    //leb decoders
    pub fn new(vec: Vec<u8>, leng: usize) -> Self
    {
        Self
        {
            byte_vec: vec,
            loc: 0,
            len: leng,
        }
    }
    pub fn leb_toi32(&mut self) -> i32
    {
        let mut decoded: i32 = 0;
        let mut shifter = 0;
        loop 
        {
            if self.loc >= self.len
            {
                panic!("Vec overflow!") //Temp will add robust error handling later
            }
            let byte = self.byte_vec[self.loc];
            self.loc += 1;
            let mut shifty = (byte & 0x7F) as i32;
            shifty <<= shifter;
            shifter += 7;
            decoded |= shifty;
            if (byte & 0x80) == 0
            {
                if shifter < 32 && (byte & 0x40) != 0
                {
                    decoded |= !0 << shifter;
                }
                return decoded;
            }
        }
    }
    pub fn leb_toi64(&mut self) -> i64
    {
        let mut decoded: i64 = 0;
        let mut shifter: i64 = 0;
        loop 
        {
            if self.loc >= self.len
            {
                panic!("Vec overflow!") //Temp will add robust error handling later
            }
            let byte = self.byte_vec[self.loc];
            self.loc += 1;
            let mut shifty = (byte & 0x7F) as i64;
            shifty <<= shifter;
            shifter += 7;
            decoded |= shifty;
            if (byte & 0x80) == 0
            {
                if shifter < 64 && (byte & 0x40) != 0
                {
                    decoded |= !0 << shifter;
                }
                return decoded;
            }     
        }
    }
    pub fn leb_tou32(&mut self) -> u32
    {
        let mut decoded: u32 = 0;
        let mut shifter: u32 = 0;
        let mut shifty = 0;
        loop 
        {
            if self.loc >= self.len || shifter > 35
            {
                panic!("Vec Overflow") //Temp will replace
            }
            let byte = self.byte_vec[self.loc];
            self.loc += 1;
            shifty = (byte & 0x7F) as u32;
            decoded |= shifty << shifter;
            if(byte & 0x80) == 0
            {
                return decoded;
            }
            shifter += 7;

        }
    }
    pub fn parse_wasm(&mut self) -> Module
    {
        let mut module =  Module::new();
        self.loc = 8;
        while self.loc < self.len
        {
            let sec = self.byte_vec[self.loc];
            self.loc += 1;
            let mut size = self.leb_tou32() as usize;
            let mut start = self.loc;
            size += start;

            match sec
            {
                1 => {
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
                3 => {
                    let mut count = self.leb_tou32();
                    while count > 0
                    {
                        let ind = self.leb_tou32();
                        module.fnid.push(ind);
                        count -= 1;
                    } 
                }
                5 =>{
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
                7 => {
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
                10 => {
                    let mut count = self.leb_tou32() as usize;
                    while count > 0
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
                        count -= 1;
                        module.fcce.push(Function{vars, code});
                    }
                }
                _ => self.loc = size, // Skipping other sections until implemented
            }


        }
        module
    }
}

pub fn wasm_engine(file_path: &Path) -> bool
{
    //execute wasm file.
    let wasm_binary:Vec<u8> = fs::read(file_path).expect("Wasm file could not be read");
    let magic_num: Vec<u8> = vec![0x00, 0x61, 0x73, 0x6D];
    let version: Vec<u8> = vec![0x01, 0x00, 0x00, 0x00];
    if wasm_binary.len() < 8
    {
        println!("Invalid file");
        return false;
    }
    if magic_num != wasm_binary[0..4] || version != wasm_binary[4..8]
    {
        println!("Invalid file");
        return false;
    }
    let leng = wasm_binary.len();
    let mut cursor = Curse::new(wasm_binary, leng);
    let mut module = cursor.parse_wasm();
    let mut wasm_runner = Runtime::new(module);
    wasm_runner.run_prog();
    true
}