//Note to readers: I have added some of the functionality for wasm 3.0 but it is currently commented out
use std::{fs, path::Path};
use super::wasm_module::*;
use super::build_runtime::*;
pub struct Curse
{
    byte_vec: Vec<u8>,
    loc: usize,
    len: usize,
    size: usize,
}
impl Curse
{
    pub fn new(vec: Vec<u8>, leng: usize) -> Self
    {
        Self
        {
            byte_vec: vec,
            loc: 0,
            len: leng,
            size: 0,
        }
    }
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
            0x11 => 
            {
                let c = Code::CallIndirect(self.leb_tou32());
                let index = self.byte_vec[self.loc];//curr not used for wasm 3.0
                self.loc += 1;

                //Remove when wasm 3.0
                assert!(index == 0x00);

                c
            }
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
            0x3F => {
                let res = self.byte_vec[self.loc];
                self.loc += 1;
                assert!(res == 0x00);
                Code::MemorySize
            },
            0x40 => {
                let res = self.byte_vec[self.loc];
                self.loc += 1;
                assert!(res == 0x00);   
                Code::MemoryGrow
            }
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
        let mut tot = 0;
        for i in 0..4
        {
            let byte = self.byte_vec[self.loc] as u32;
            self.loc += 1;
            tot |= byte << (8 * i);
        }
        f32::from_bits(tot)
    }
    pub fn get_code_exp(&mut self) -> Vec<Code> //Function originally part of section 11 repurposed for general use as helper function converts bytes to opcodes.
    {    //For wasm 3.0 not needed yet.   
        let mut code: Vec<Code> = Vec::new();
        let mut mbyt = self.byte_vec[self.loc];
        assert!(mbyt != 0x0b);
        while mbyt != 0x0b
        { 
            self.loc += 1;
            match mbyt
            {
                0x23 => code.push(Code::GlobalGet(self.leb_tou32())),
                0x41 => code.push(Code::I32Const(self.leb_toi32())),
                0x42 => code.push(Code::I64Const(self.leb_toi64())),
                0x43 => code.push(Code::F32Const(self.leb_tof32())),
                0x44 => code.push(Code::F64Const(self.leb_tof64())),
                //For the extended segss using glob! 
                0x6a => {
                    if code.is_empty(){panic!("Parser ecountered Arithmatic opcode first in Element Segment Invalid Module");}
                    code.push(Code::I32Add);
                }
                0x6b => {
                    if code.is_empty(){panic!("Parser ecountered Arithmatic opcode first in Element Segment Invalid Module");}
                    code.push(Code::I32Add);
                }
                0x6c =>
                {
                    if code.is_empty(){panic!("Parser ecountered Arithmatic opcode first in Element Segment Invalid Module");}
                    code.push(Code::I32Mul);
                }
                0x7c =>
                {
                    if code.is_empty(){panic!("Parser ecountered Arithmatic opcode first in Element Segment Invalid Module");}
                    code.push(Code::I64Add);
                }
                0x7d => 
                {
                    if code.is_empty(){panic!("Parser ecountered Arithmatic opcode first in Element Segment Invalid Module");}
                    code.push(Code::I64Sub);
                }
                0x7e =>
                {
                    if code.is_empty(){panic!("Parser ecountered Arithmatic opcode first in Element Segment Invalid Module");}
                    code.push(Code::I64Mul);
                }
                //0xd0 => code.push(Code::RNull(self.leb_tou32())), Not Implemented added for later!
                //0xd2 => code.push(Code::RFunc(self.leb_tou32())),       
                _ => panic!("Invalid opcode Section 11 Byte: "),                             
            }
            mbyt = self.byte_vec[self.loc];
            assert!(self.loc < self.size);
        }
        self.loc += 1;
        code
    }
    fn get_fvec_data(&mut self) -> Vec<u32>
    {
        let mut fvec: Vec<u32> = Vec::new();
        let mut count = self.leb_tou32();
        while count > 0
        {
            fvec.push(self.leb_tou32());
            count -= 1;
        }
        fvec
    }
    fn get_off(&mut self) -> Code
    {
        let mut mbyt = self.byte_vec[self.loc];
        self.loc += 1;
        let code = match mbyt
        {
            //0x23 => Code::GlobalGet(self.leb_tou32()),
            0x41 => Code::I32Const(self.leb_toi32()),
            //0x42 => Code::I64Const(self.leb_toi64()),    
            _ => panic!("Invalid opcode Section 11 Byte: {}", self.loc),                             
        };
        let end = self.byte_vec[self.loc];
        self.loc += 1;
        assert!(end == 0x0b);
        code
    }
    fn get_tab(&mut self) -> Tab
    {
        let mut tabmin: u32 = 0;
        let mut tabmax = None;
        //let mut tabmin64 = None;
        //let mut tabmax64 = None;
        let typ = self.byte_vec[self.loc];
        self.loc += 1;
        
        //wasm 1.0
        assert!(typ == 0x70);

        let flag = self.byte_vec[self.loc];
        self.loc += 1;
        match flag {
            0x00 => tabmin = self.leb_tou32(),
            0x01 => {
                tabmin = self.leb_tou32();
                tabmax = Some(self.leb_tou32());
            }
            /*0x02 => tabmin64 = Some(self.leb_toi64()),
            0x03 => {
                tabmin64 = Some(self.leb_toi64());
                tabmax64 = Some(self.leb_toi64());
            }*/
            _ => panic!("Invalid flag Table Section 4 Byte: {}", self.loc),
                
        }
        Tab{typ, flag, tabmin, tabmax/*, tabmin64,tabmax64*/}
    }
    fn get_mem(&mut self) -> MemoIn
    {
        let flag = self.byte_vec[self.loc];
        self.loc += 1;
        let memmin = self.leb_tou32();
        let mut memmax = None;
        if flag & 0x01 != 0
        {
            memmax = Some(self.leb_tou32());
        } 
        MemoIn{flag, memmin, memmax}
    }
    pub fn leb_tof64(&mut self) -> f64
    {
        let mut tot= 0;
        for i in 0..8
        {
            let byte = self.byte_vec[self.loc] as u64;
            self.loc += 1;
            tot |= byte << (i * 8);
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
            self.size = self.loc + self.leb_tou32() as usize;
            let mut start = self.loc;
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
                        let mut tab = None;
                        let mut mem = None;
                        let mut glob = None;
                        let mod_len = self.leb_tou32() as usize;
                        let modname:String = String::from_utf8(self.byte_vec[self.loc..self.loc+mod_len].to_vec()).unwrap();
                        self.loc += mod_len;
                        let name_len = self.leb_tou32() as usize;
                        let impname:String = String::from_utf8(self.byte_vec[self.loc..self.loc+name_len].to_vec()).unwrap();
                        self.loc += name_len;
                        let typ = self.byte_vec[self.loc];
                        self.loc += 1;
                        match typ
                        {
                            0x00 =>{
                                let index =  Some(self.leb_tou32());
                                module.imps.push(Import{
                                    modname,
                                    impname,
                                    index,
                                    imptyp: ExpTyp::Func,
                                    glob,
                                    mem,
                                    tab,
                                });
                                module.imports += 1;
                            } 
                            0x01 => 
                            {
                                tab = Some(self.get_tab());
                                module.imps.push(Import{
                                    modname,
                                    impname,
                                    index: None,
                                    imptyp: ExpTyp::Table,
                                    glob,
                                    mem,
                                    tab,
                                });
                            }
                            0x02 => {

                                module.imps.push(Import{
                                    modname,
                                    impname,
                                    index: None,
                                    imptyp: ExpTyp::Memory,
                                    glob,
                                    mem,
                                    tab,
                                });
                            },
                            0x03 => {
                                let typ = decode_byte(self.byte_vec[self.loc]).unwrap();
                                self.loc += 1;
                                let mutcheck = self.byte_vec[self.loc];
                                self.loc += 1;
                                let mut is_mut = false;
                                if mutcheck != 0{is_mut = true;}
                                module.imps.push(Import{
                                    modname,
                                    impname,
                                    index: None,
                                    imptyp: ExpTyp::Global,
                                    glob: Some(ShortGlobal{typ, is_mut}),
                                    mem,
                                    tab,
                                });
                            },
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
                        module.tabs.push(self.get_tab());
                        count -= 1;
                    }
                }
                5 =>{       //Memmory
                    let mut count = self.leb_tou32();
                    while count > 0
                    {
                        module.memy.push(self.get_mem());
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
                        self.loc += 1;
                        let mut ismut = false;
                        if mutcheck != 0 {ismut = true;}
                        let code = self.set_code();
                        let end = self.byte_vec[self.loc];
                        count -= 1;
                        module.glob.push(Global{typ, ismut, code,})
                    }
                }
                7 => {      //Exports
                    let mut count = self.leb_tou32();
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
                9 => {      //Elements
                    let mut count = self.leb_tou32();
                    while count > 0
                    {
                        let mut elmtyp = None;
                        let mut elmoff ;
                        let mut fvec = Vec::new();
                        let tabid = self.leb_tou32();
                        match tabid {
                            0x00 => {
                                elmoff = self.get_off();
                                fvec = self.get_fvec_data();
                            }/* for wasm 3.0 commented for now
                            0x01 => {
                                elmtyp = match self.byte_vec[self.loc]{
                                    0x00 => Some(0x00),
                                    _ => panic!("Improper Element Type Byte must be 0x00"),
                                };
                                self.loc += 1;
                                fvec = self.get_fvec_data();
                            }
                            0x02 => {
                                fvec.push(self.leb_tou32());
                                elmcode.push(self.get_off());
                                elmtyp = match self.byte_vec[self.loc]{
                                    0x00 => Some(0x00),
                                    _ => panic!("Improper Element Type Byte must be 0x00"),
                                };
                                self.loc += 1;
                                fvec.extend(self.get_fvec_data());
                            }
                            0x03 => {
                                elmtyp = match self.byte_vec[self.loc]{
                                    0x00 => Some(0x00),
                                    _ => panic!("Improper Element Type Byte must be 0x00"),
                                };
                                self.loc += 1;
                                fvec = self.get_fvec_data();
                            }
                            0x04 => {
                                elmcode.push(self.get_off());
                                fvec = self.get_fvec_data();
                                elmtyp = None;
                            }
                            0x05 => {
                                elmtyp = Some(self.byte_vec[self.loc]);
                                self.loc += 1;
                                elmcode = self.get_code_exp();
                            }
                            0x06 => {
                                fvec.push(self.leb_tou32());
                                elmcode.push(self.get_off());
                                elmtyp = Some(self.byte_vec[self.loc]);
                                self.loc += 1;
                                elmcode.extend(self.get_code_exp());
                            }
                            0x07 => {
                                elmtyp = Some(self.byte_vec[self.loc]);
                                self.loc += 1;
                                elmcode = self.get_code_exp();
                            } */
                            _ => panic!("Invalid wasm file: Flag byte must be 0x00 Elements Byte Location: {}", self.loc),
                        };
                        module.elms.push(Element{tabid, elmtyp, elmoff, fvec});
                        count -= 1;
                    }
                }
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
                        assert!(self.loc == cend);
                    }
                }
                11 => {     //MemInit
                    let mut count = self.leb_tou32();
                    while count > 0
                    {
                        let flags = self.leb_tou32(); // for wasm 3.0 still needed for wasm 1.0 for 0x00 flag

                        //remove for wasm 3.0
                        assert!(flags == 0x00);


                        //let memtyp: MemTyp;
                        //let mut code = Vec::new();
                        /*if flags & 0x01 != 0 
                        {
                            memtyp = MemTyp::Immediate;
                            code = self.get_code_exp();
                        }
                        else {memtyp = MemTyp::Waiting;}*/
                        let code = self.get_off();
                        let mut datalen = self.leb_tou32() as usize;
                        let mut dvec: Vec<u8> = Vec::new();
                        while datalen > 0
                        {
                            dvec.push(self.byte_vec[self.loc]);
                            self.loc += 1;
                            datalen -= 1;
                        }
                        module.mmsg.push(MemSeg{/*memtyp,*/ code, dvec});
                        count -= 1;
                    }
                }
                /*12 => {     //MemInit Count
                    module.memcount = self.leb_tou32();
                }*/

                _ => self.loc = self.size, // Skipping other sections until implemented
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
        let mut tcurs = Curse::new(vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00], 8);
        let tmod = tcurs.parse_wasm();

        assert!(tmod.typs.is_empty());
        assert!(tmod.fcce.is_empty());
        assert!(tmod.exps.is_empty());
        assert!(tmod.imports == 0);
        assert!(tmod.strt.is_none());
    }
    #[test]//8.2
    #[should_panic]
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
