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

        }
    }
    pub fn leb_tof32(&mut self) -> f32
    {
        let mut bits = Vec::new();
        for i in 0..4
        {
            let byte = self.byte_vec[self.loc];
            self.loc += 1;
            bits.push((byte as u32) << (8 * i));
        }
        let mut tot = 0;
        let init = bits[0];
        for i in bits
        {
            if i == init
            {
                tot = i;
            }
            else{
                tot |= i
            }
        }
        f32::from_bits(tot)
    }
    pub fn leb_tof64(&mut self) -> f64
    {
        let mut bits = Vec::new();
        for i in 0..8
        {
            let byte = self.byte_vec[self.loc];
            self.loc += 1;
            bits.push((byte as u64) << (8 * i));
        }
        let mut tot = 0;
        let init = bits[0];
        for i in bits
        {
            if i == init
            {
                tot = i;
            }
            else{
                tot |= i
            }
        }
        f64::from_bits(tot)
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

                                });
                                module.imports += 1;
                            } 
                            //0x01 => ExpTyp::Table,
                            //0x02 => ExpTyp::Memory,
                            //0x03 => ,
                            _ => panic!("Crit Byte error!"),

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
// Additional tests to add to the existing #[cfg(test)] mod tests section in wasm_engine.rs

    // Tests for Curse::new
    #[test] //9.1
    fn test_curse_new() {
        let bytes = vec![0x01, 0x02, 0x03, 0x04];
        let curse = Curse::new(bytes.clone(), 4);
        assert_eq!(curse.byte_vec, bytes);
        assert_eq!(curse.loc, 8); // After magic number and version
        assert_eq!(curse.len, 4);
    }

    #[test] //9.2
    fn test_curse_new_empty() {
        let bytes = vec![];
        let curse = Curse::new(bytes.clone(), 0);
        assert_eq!(curse.byte_vec.len(), 0);
        assert_eq!(curse.loc, 8);
        assert_eq!(curse.len, 0);
    }

    // Tests for set_code - Flow control instructions
    #[test] //10.1
    fn test_set_code_unreachable() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x00], 9);
        curse.loc = 8; // Move to actual code byte
        let code = curse.set_code();
        assert!(matches!(code, Code::Unreachable));
    }

    #[test] //10.2
    fn test_set_code_nop() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x01], 9);
        curse.loc = 8;
        let code = curse.set_code();
        assert!(matches!(code, Code::Nop));
    }

    #[test] //10.3
    fn test_set_code_block() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x02, 0x40], 10);
        curse.loc = 8;
        let code = curse.set_code();
        assert!(matches!(code, Code::Block(_)));
    }

    #[test] //10.4
    fn test_set_code_loop() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x03, 0x40], 10);
        curse.loc = 8;
        let code = curse.set_code();
        assert!(matches!(code, Code::Loop(_)));
    }

    #[test] //10.5
    fn test_set_code_if() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x04, 0x40], 10);
        curse.loc = 8;
        let code = curse.set_code();
        assert!(matches!(code, Code::If(_)));
    }

    #[test] //10.6
    fn test_set_code_else() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x05], 9);
        curse.loc = 8;
        let code = curse.set_code();
        assert!(matches!(code, Code::Else));
    }

    #[test] //10.7
    fn test_set_code_end() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x0B], 9);
        curse.loc = 8;
        let code = curse.set_code();
        assert!(matches!(code, Code::End));
    }

    #[test] //10.8
    fn test_set_code_return() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x0F], 9);
        curse.loc = 8;
        let code = curse.set_code();
        assert!(matches!(code, Code::Return));
    }

    // Tests for set_code - Variable instructions
    #[test] //11.1
    fn test_set_code_local_get() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x20, 0x00], 10);
        curse.loc = 8;
        let code = curse.set_code();
        match code {
            Code::LocalGet(idx) => assert_eq!(idx, 0),
            _ => panic!("Expected LocalGet"),
        }
    }

    #[test] //11.2
    fn test_set_code_local_set() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x21, 0x01], 10);
        curse.loc = 8;
        let code = curse.set_code();
        match code {
            Code::LocalSet(idx) => assert_eq!(idx, 1),
            _ => panic!("Expected LocalSet"),
        }
    }

    #[test] //11.3
    fn test_set_code_local_tee() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x22, 0x02], 10);
        curse.loc = 8;
        let code = curse.set_code();
        match code {
            Code::LocalTee(idx) => assert_eq!(idx, 2),
            _ => panic!("Expected LocalTee"),
        }
    }

    #[test] //11.4
    fn test_set_code_global_get() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x23, 0x00], 10);
        curse.loc = 8;
        let code = curse.set_code();
        match code {
            Code::GlobalGet(idx) => assert_eq!(idx, 0),
            _ => panic!("Expected GlobalGet"),
        }
    }

    #[test] //11.5
    fn test_set_code_global_set() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x24, 0x00], 10);
        curse.loc = 8;
        let code = curse.set_code();
        match code {
            Code::GlobalSet(idx) => assert_eq!(idx, 0),
            _ => panic!("Expected GlobalSet"),
        }
    }

    // Tests for set_code - Memory instructions
    #[test] //12.1
    fn test_set_code_i32_load() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x28, 0x00, 0x04], 11);
        curse.loc = 8;
        let code = curse.set_code();
        match code {
            Code::I32Load(off) => assert_eq!(off, 4),
            _ => panic!("Expected I32Load"),
        }
    }

    #[test] //12.2
    fn test_set_code_i64_load() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x29, 0x00, 0x08], 11);
        curse.loc = 8;
        let code = curse.set_code();
        match code {
            Code::I64Load(off) => assert_eq!(off, 8),
            _ => panic!("Expected I64Load"),
        }
    }

    #[test] //12.3
    fn test_set_code_f32_load() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x2A, 0x00, 0x00], 11);
        curse.loc = 8;
        let code = curse.set_code();
        match code {
            Code::F32Load(off) => assert_eq!(off, 0),
            _ => panic!("Expected F32Load"),
        }
    }

    #[test] //12.4
    fn test_set_code_f64_load() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x2B, 0x00, 0x00], 11);
        curse.loc = 8;
        let code = curse.set_code();
        match code {
            Code::F64Load(off) => assert_eq!(off, 0),
            _ => panic!("Expected F64Load"),
        }
    }

    #[test] //12.5
    fn test_set_code_i32_store() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x36, 0x00, 0x10], 11);
        curse.loc = 8;
        let code = curse.set_code();
        match code {
            Code::I32Store(off) => assert_eq!(off, 16),
            _ => panic!("Expected I32Store"),
        }
    }

    #[test] //12.6
    fn test_set_code_memory_size() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x3F], 9);
        curse.loc = 8;
        let code = curse.set_code();
        assert!(matches!(code, Code::MemorySize));
    }

    #[test] //12.7
    fn test_set_code_memory_grow() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x40], 9);
        curse.loc = 8;
        let code = curse.set_code();
        assert!(matches!(code, Code::MemoryGrow));
    }

    // Tests for set_code - Constant instructions
    #[test] //13.1
    fn test_set_code_i32_const() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x41, 0x2A], 10);
        curse.loc = 8;
        let code = curse.set_code();
        match code {
            Code::I32Const(val) => assert_eq!(val, 42),
            _ => panic!("Expected I32Const"),
        }
    }

    #[test] //13.2
    fn test_set_code_i64_const() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x42, 0x64], 10);
        curse.loc = 8;
        let code = curse.set_code();
        match code {
            Code::I64Const(val) => assert_eq!(val, 100),
            _ => panic!("Expected I64Const"),
        }
    }

    // Tests for set_code - Comparison instructions
    #[test] //14.1
    fn test_set_code_i32_eqz() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x45], 9);
        curse.loc = 8;
        let code = curse.set_code();
        assert!(matches!(code, Code::I32Eqz));
    }

    #[test] //14.2
    fn test_set_code_i32_eq() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x46], 9);
        curse.loc = 8;
        let code = curse.set_code();
        assert!(matches!(code, Code::I32Eq));
    }

    #[test] //14.3
    fn test_set_code_i32_ne() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x47], 9);
        curse.loc = 8;
        let code = curse.set_code();
        assert!(matches!(code, Code::I32Ne));
    }

    #[test] //14.4
    fn test_set_code_i32_lts() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x48], 9);
        curse.loc = 8;
        let code = curse.set_code();
        assert!(matches!(code, Code::I32LtS));
    }

    #[test] //14.5
    fn test_set_code_i64_eqz() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x50], 9);
        curse.loc = 8;
        let code = curse.set_code();
        assert!(matches!(code, Code::I64Eqz));
    }

    // Tests for set_code - Arithmetic instructions
    #[test] //15.1
    fn test_set_code_i32_add() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x6A], 9);
        curse.loc = 8;
        let code = curse.set_code();
        assert!(matches!(code, Code::I32Add));
    }

    #[test] //15.2
    fn test_set_code_i32_sub() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x6B], 9);
        curse.loc = 8;
        let code = curse.set_code();
        assert!(matches!(code, Code::I32Sub));
    }

    #[test] //15.3
    fn test_set_code_i32_mul() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x6C], 9);
        curse.loc = 8;
        let code = curse.set_code();
        assert!(matches!(code, Code::I32Mul));
    }

    #[test] //15.4
    fn test_set_code_i64_add() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x7C], 9);
        curse.loc = 8;
        let code = curse.set_code();
        assert!(matches!(code, Code::I64Add));
    }

    #[test] //15.5
    fn test_set_code_f32_add() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x92], 9);
        curse.loc = 8;
        let code = curse.set_code();
        assert!(matches!(code, Code::F32Add));
    }

    // Tests for set_code - Stack instructions
    #[test] //16.1
    fn test_set_code_drop() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x1A], 9);
        curse.loc = 8;
        let code = curse.set_code();
        assert!(matches!(code, Code::Drop));
    }

    #[test] //16.2
    fn test_set_code_select() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x1B], 9);
        curse.loc = 8;
        let code = curse.set_code();
        assert!(matches!(code, Code::Select));
    }

    // Tests for leb_toi64
    #[test] //17.1
    fn test_leb_toi64_positive() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0xE5, 0x8E, 0x26], 11);
        curse.loc = 8;
        let val = curse.leb_toi64();
        assert_eq!(val, 624485);
    }

    #[test] //17.2
    fn test_leb_toi64_negative() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x7F], 9);
        curse.loc = 8;
        let val = curse.leb_toi64();
        assert_eq!(val, -1);
    }

    // Tests for parse_wasm with different sections
    #[test] //18.1
    fn test_parse_wasm_with_type_section() {
        // Minimal WASM with a type section defining one function type
        let bytes = vec![
            0x00, 0x61, 0x73, 0x6D, // magic
            0x01, 0x00, 0x00, 0x00, // version
            0x01, // type section
            0x04, // section size
            0x01, // 1 type
            0x60, 0x00, 0x00, // func type: () -> ()
        ];
        let mut curse = Curse::new(bytes, 15);
        let module = curse.parse_wasm();
        assert_eq!(module.typs.len(), 1);
    }

    #[test] //18.2
    fn test_parse_wasm_with_memory_section() {
        let bytes = vec![
            0x00, 0x61, 0x73, 0x6D, // magic
            0x01, 0x00, 0x00, 0x00, // version
            0x05, // memory section
            0x03, // section size
            0x01, // 1 memory
            0x00, 0x01, // min=1, no max
        ];
        let mut curse = Curse::new(bytes, 14);
        let module = curse.parse_wasm();
        assert_eq!(module.memy.len(), 1);
        assert_eq!(module.memy[0].0, 1);
        assert_eq!(module.memy[0].1, None);
    }

    #[test] //18.3
    fn test_parse_wasm_with_export_section() {
        let bytes = vec![
            0x00, 0x61, 0x73, 0x6D, // magic
            0x01, 0x00, 0x00, 0x00, // version
            0x07, // export section
            0x08, // section size
            0x01, // 1 export
            0x04, // name length
            0x6D, 0x61, 0x69, 0x6E, // "main"
            0x00, // function export
            0x00, // function index 0
        ];
        let mut curse = Curse::new(bytes, 19);
        let module = curse.parse_wasm();
        assert_eq!(module.exps.len(), 1);
        assert_eq!(module.exps[0].name, "main");
    }

    // Tests for wasm_engine function
    #[test] //19.1
    fn test_wasm_engine_invalid_magic() {
        use std::io::Write;
        use std::path::PathBuf;
        
        let temp_file = PathBuf::from("/tmp/test_invalid_magic.wasm");
        let mut file = std::fs::File::create(&temp_file).unwrap();
        file.write_all(&[0xFF, 0xFF, 0xFF, 0xFF, 0x01, 0x00, 0x00, 0x00]).unwrap();
        
        let result = wasm_engine(&temp_file);
        assert_eq!(result, false);
        
        std::fs::remove_file(&temp_file).ok();
    }

    #[test] //19.2
    fn test_wasm_engine_invalid_version() {
        use std::io::Write;
        use std::path::PathBuf;
        
        let temp_file = PathBuf::from("/tmp/test_invalid_version.wasm");
        let mut file = std::fs::File::create(&temp_file).unwrap();
        file.write_all(&[0x00, 0x61, 0x73, 0x6D, 0x02, 0x00, 0x00, 0x00]).unwrap();
        
        let result = wasm_engine(&temp_file);
        assert_eq!(result, false);
        
        std::fs::remove_file(&temp_file).ok();
    }

    #[test] //19.3
    fn test_wasm_engine_too_short() {
        use std::io::Write;
        use std::path::PathBuf;
        
        let temp_file = PathBuf::from("/tmp/test_too_short.wasm");
        let mut file = std::fs::File::create(&temp_file).unwrap();
        file.write_all(&[0x00, 0x61, 0x73]).unwrap();
        
        let result = wasm_engine(&temp_file);
        assert_eq!(result, false);
        
        std::fs::remove_file(&temp_file).ok();
    }

    #[test] //19.4
    fn test_wasm_engine_valid_empty() {
        use std::io::Write;
        use std::path::PathBuf;
        
        let temp_file = PathBuf::from("/tmp/test_valid_empty.wasm");
        let mut file = std::fs::File::create(&temp_file).unwrap();
        file.write_all(&[0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00]).unwrap();
        
        let result = wasm_engine(&temp_file);
        assert_eq!(result, true);
        
        std::fs::remove_file(&temp_file).ok();
    }

    // Additional edge case tests
    #[test] //20.1
    #[should_panic(expected = "Cursor Bounds Breached!!!")]
    fn test_set_code_out_of_bounds() {
        let mut curse = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00], 8);
        curse.loc = 8; // At the end
        curse.set_code(); // Should panic
    }

    #[test] //20.2
    fn test_multiple_globals_parsing() {
        let bytes = vec![
            0x00, 0x61, 0x73, 0x6D, // magic
            0x01, 0x00, 0x00, 0x00, // version
            0x06, // global section
            0x0A, // section size
            0x02, // 2 globals
            0x7F, 0x00, 0x41, 0x2A, 0x0B, // i32, immutable, i32.const 42, end
            0x7E, 0x01, 0x42, 0x64, 0x0B, // i64, mutable, i64.const 100, end
        ];
        let mut curse = Curse::new(bytes, 21);
        let module = curse.parse_wasm();
        assert_eq!(module.glob.len(), 2);
        assert_eq!(module.glob[0].ismut, false);
        assert_eq!(module.glob[1].ismut, true);
    }

    #[test] //20.3
    fn test_function_section_parsing() {
        let bytes = vec![
            0x00, 0x61, 0x73, 0x6D, // magic
            0x01, 0x00, 0x00, 0x00, // version
            0x03, // function section
            0x03, // section size
            0x02, // 2 functions
            0x00, 0x01, // type indices
        ];
        let mut curse = Curse::new(bytes, 15);
        let module = curse.parse_wasm();
        assert_eq!(module.fnid.len(), 2);
        assert_eq!(module.fnid[0], 0);
        assert_eq!(module.fnid[1], 1);
    }

    #[test] //20.4
    fn test_start_section_parsing() {
        let bytes = vec![
            0x00, 0x61, 0x73, 0x6D, // magic
            0x01, 0x00, 0x00, 0x00, // version
            0x08, // start section
            0x01, // section size
            0x05, // start function index
        ];
        let mut curse = Curse::new(bytes, 12);
        let module = curse.parse_wasm();
        assert_eq!(module.strt, Some(5));
    }

    #[test] //20.5
    fn test_table_section_parsing() {
        let bytes = vec![
            0x00, 0x61, 0x73, 0x6D, // magic
            0x01, 0x00, 0x00, 0x00, // version
            0x04, // table section
            0x04, // section size
            0x01, // 1 table
            0x70, 0x00, 0x0A, // funcref, min=10, no max
        ];
        let mut curse = Curse::new(bytes, 15);
        let module = curse.parse_wasm();
        assert_eq!(module.tabs.len(), 1);
        assert_eq!(module.tabs[0].tabmin, 10);
        assert_eq!(module.tabs[0].tabmax, None);
    }

    
  
    
