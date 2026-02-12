use core::panic;
use super::wasm_module::*;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
#[derive(Clone, Debug)]
pub enum StackTypes
{
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}
#[derive(Clone)]
pub struct StackCalls
{
    pub fnid: usize,
    pub code: Vec<Code>,
    pub loc: usize,
    pub vars: Vec<StackTypes>,
}
#[derive(Clone)]
pub struct GlobsGlobal
{
    typ: StackTypes, 
    ismut: bool,
}
#[derive(Clone)]
pub struct Runtime
{
    pub module: Module,
    pub mem: Vec<u8>,
    pub memmin: u32,
    pub memmax: Option<u32>,
    pub call_stack: Vec<StackCalls>, 
    pub value_stack: Vec<StackTypes>,
    pub flow_stack: Vec<FlowCode>,
    pub globs: Vec<GlobsGlobal>,
    pub functab: Vec<Option<u32>>,

}
#[derive(Clone)]
pub enum FlowType
{
    If,
    Block,
    Loop,
}
#[derive(Clone)]
pub struct FlowCode
{
    flow_type: FlowType,
    break_tar: usize,
    size: usize,
    ret_typ: Option<TypeBytes>
    
}
impl Runtime
{
    pub fn new(module: Module) -> Self
    {   
        //Imports 
        //Will Add Soon
        //Memory Allocation
        let mut memmin: u32 = 0;
        let mut memmax: Option<u32> = None;
        let mut mimpbool = false;
        for mut i in module.imps.clone()
        {
            if i.mem.is_some()
            {
                memmax = i.mem.as_mut().unwrap().memmax;
                memmin = i.mem.unwrap().memmin;
                mimpbool = true;
                break;
            }
        }
        if !mimpbool{
            
            if !module.memy.is_empty()
            {
                memmin = module.memy[0].memmin;
                memmax = module.memy[0].memmax;
            }
            else 
            {
                memmin = 0;
                memmax = None;    
            }
        }
        let mut bytes =  memmin as usize;
        bytes *= 65536;
        let mut memvec = vec![0; bytes];
        //Loading mem
        for mems in &module.mmsg
        {
            let off = match mems.code{
                Code::I32Const(val) => val,
                _ => panic!("Invalid offset type loading memory"),
            } as usize;
            assert!((off + mems.dvec.len() <= memvec.len()));
            memvec[off..off + mems.dvec.len()].copy_from_slice(&mems.dvec);
        }
        let mut globs: Vec<GlobsGlobal> = Vec::new(); 
        for global in &module.glob 
        {
            let mut gval: StackTypes = match global.code
            {
                Code::I32Const(cons) => StackTypes::I32(cons),
                Code::I64Const(cons) => StackTypes::I64(cons),
                Code::F32Const(cons) => StackTypes::F32(cons),
                Code::F64Const(cons) =>  StackTypes::F64(cons),
                _ => panic!("Invalid Global"),
            };

            let ismut = global.ismut;
            globs.push(GlobsGlobal{typ: gval, ismut});
        }
        let mut functab: Vec<Option<u32>> = Vec::new();
        if !module.tabs.is_empty()
        {
            functab = vec![None; module.tabs[0].tabmin as usize];
        }
        for elm in &module.elms
        {
            let mut off = match elm.elmoff
            {
                Code::I32Const(val) => val,
                _ => panic!("Invalid Constant Elements"),
            } as usize;
            assert!(off + elm.fvec.len() <= functab.len());
            for byts in &elm.fvec
            {
                functab[off] = Some(*byts);
                off +=1;
            }
        }
        Runtime{module, functab, mem: memvec, memmin, memmax, call_stack: Vec::new(), value_stack: Vec::new(), flow_stack: Vec::new(), globs,}
    }
    pub fn run_prog(&mut self)
    {
        let pstring = format!("{}{}{}", "./wasm_files/", self.module.name, ".txt");
        let path = Path::new(&pstring);
        let mut wasfile = File::create(path).expect("Log file could not be created");
        //simple_logging::log_to_file("wasm.log", log::LevelFilter::Info);
        //log::info!("Wasm running");
        if let Some(starter) = self.module.strt
        {
            let strtind = (starter - self.module.imports) as usize;
            let typin = self.module.fnid[strtind] as usize;
            let typ = &self.module.typs[typin];
            let func = &self.module.fcce[strtind];
            let mut vars = Vec::new();
            for (loc, typ) in &func.vars
            {
                let ty = match typ
                {
                    Some(typ) => typ,
                    None => panic!("typ error run_prog")
                };
                let styp = match ty
                {
                    TypeBytes::I32 => StackTypes::I32(0),
                    TypeBytes::I64 => StackTypes::I64(0),
                    TypeBytes::F32 => StackTypes::F32(0.0),
                    TypeBytes::F64 => StackTypes::F64(0.0),
                };
                for _ in 0..*loc
                {
                    vars.push(styp.clone());
                }
            }
            self.call_stack.push(StackCalls { fnid: strtind, code: func.code.clone(), loc: 0, vars,});
        }
        else
        {
            let func = &self.module.fcce[self.module.imports as usize];
            let mut vars = Vec::new();
            for arg in &self.module.typs[self.module.imports as usize].args
            {
                let ar = match arg
                {
                    Some(TypeBytes::I32) => StackTypes::I32(0),
                    Some(TypeBytes::I64) => StackTypes::I64(0),
                    Some(TypeBytes::F32) => StackTypes::F32(0.0),
                    Some(TypeBytes::F64) => StackTypes::F64(0.0),
                    None => panic!("Invalid argument start function"), 
                };
                    vars.push(ar);
            }
            for (loc, typ) in &func.vars
            {
                let ty = match typ
                {
                    Some(val) => val,
                    None => panic!("Call vars err"),
                };
                let var = match ty 
                {
                    TypeBytes::I32 => StackTypes::I32(0),
                    TypeBytes::I64 => StackTypes::I64(0),
                    TypeBytes::F32 => StackTypes::F32(0.0),
                    TypeBytes::F64 => StackTypes::F64(0.0),
                };

                for _ in 0..*loc
                {
                    vars.push(var.clone())
                }
            }
            self.call_stack.push(StackCalls { fnid: 0, code: func.code.clone(), loc: 0, vars});
        }
        let mut incount = 0;
        'run:
        loop {
            let call = match self.call_stack.last_mut()
            {
                Some(caller) => caller,
                None => {
                    //log::info!("End of wasm");
                    return;
                }
            };
            if call.loc >= call.code.len()
            {
                let turn = self.value_stack.pop();
                self.call_stack.pop();
                if self.call_stack.is_empty()
                {
                    return;
                }
                if let Some(turner) = turn {self.value_stack.push(turner)}
                continue 'run;
            }
            let code = call.code[call.loc].clone();
            call.loc += 1;
            match code
            {
                //flow
                Code::Unreachable => panic!("wasm module reached unreachable instruction"),
                Code::Nop => (), //instruction is a placeholder in wasm
                Code::Block(typ) => self.flow_stack.push(FlowCode{flow_type: FlowType::Block, break_tar: call.code.len() - 1, size: self.value_stack.len(), ret_typ: typ}),
                Code::Loop(typ) => self.flow_stack.push(FlowCode{ flow_type: FlowType::Loop, break_tar: call.code.len(), size: self.value_stack.len(), ret_typ: typ,}),    
                //Code::If(typ) => self.flow_stack.push(FlowCode{flow_type: FlowType::If, break_tar: , size: (), ret_typ: () }),
//                Code::Else => //log::info!("Else"),
                Code::Br(us) => 
                {
                    while self.flow_stack.len() >= us as usize
                    {
                        self.flow_stack.pop();
                    }

                }
                Code::BrIf(us) => 
                {
                    match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(boo)) =>
                        {
                            if boo == 0
                            {
                                
                            }
                            else 
                            {
                                while self.flow_stack.len() >= us as usize
                                {
                                    self.flow_stack.pop();
                                }
                            }
                        }
                        _ => panic!("Expected I32 from stack BrIF"),
                    }
                },
                //Code::BrTable => (),
                Code::Return | Code::End =>
                {
                    let turn = self.value_stack.pop();
                    self.call_stack.pop();
                    if self.call_stack.is_empty(){
                        //log::info!("Return");
                        //return turn;
                        return;
                    }
                    if let Some(val) = turn
                    {
                        //log::info!("Return: {}", val);
                        self.value_stack.push(val);
                    }

                },
                Code::Call(ind) => 
                {
                    writeln!(&mut wasfile,"{}. Call {}", incount, ind).expect("File Write Error");
                    //log::info!("Function Call, ID: {}", ind);
                    let typind = self.module.fnid[ind as usize] as usize;
                    let typ = &self.module.typs[typind];
                    let mut cvec = Vec::new();
                    let mut itt = 0;
                    while itt < typ.args.len()
                    {
                        cvec.push(self.value_stack.pop().unwrap());
                        itt += 1;
                    }
                    cvec.reverse();
                    //make call
                    if typ.args.len() != cvec.len(){panic!("Call vec length err");}
                    let func = &self.module.fcce[(ind - self.module.imports) as usize];
                    let fcode = &func.code;
                    let mut vars = Vec::new();
                    vars.extend(cvec);
                    for (loc, typ) in &func.vars
                    {
                        let ty = match typ
                        {
                            Some(val) => val,
                            None => panic!("Call vars err"),
                        };
                        let var = match ty 
                        {
                            TypeBytes::I32 => StackTypes::I32(0),
                            TypeBytes::I64 => StackTypes::I64(0),
                            TypeBytes::F32 => StackTypes::F32(0.0),
                            TypeBytes::F64 => StackTypes::F64(0.0),
                        };

                        for _ in 0..*loc
                        {
                            vars.push(var.clone())
                        }

                    }
                    self.call_stack.push(StackCalls{ fnid: (ind - self.module.imports)as usize, code: fcode.to_vec(), loc: 0, vars});
                    continue 'run;
                },
                /*Code::CallIndirect(ind) => 
                {

                },*/
                //Args
                Code::Drop =>
                {
                    let waste = self.value_stack.pop();
                    writeln!(&mut wasfile,"{}. Drop {:?}", incount, waste).expect("File Write Error");
                },
                Code::Select =>
                {
                    let sel = match self.value_stack.pop(){
                        Some(StackTypes::I32(val)) => val,
                        _ => panic!("Stack Error Select"),
                    };
                    writeln!(&mut wasfile,"{}. Select {}", incount, sel).expect("File Write Error");
                    let val2 = self.value_stack.pop().expect("Stack Sel Fail");
                    let val1 = self.value_stack.pop().expect("Stack Sel Fail");

                    if sel != 0 {self.value_stack.push(val1);}
                    else{self.value_stack.push(val2);}
                },
                //Vars
                Code::LocalGet(loc) => {
                    let val = call.vars.get(loc as usize).unwrap().clone();
                    writeln!(&mut wasfile,"{}. Local Get({}): {:?}", incount, loc, val).expect("File Write Error");
                    self.value_stack.push(val);
                    //log::info!("Local Get: Index: {}, Value: {}", loc, val);
                },
                Code::LocalSet(loc) => {
                    let to_stack = self.value_stack.pop().unwrap();
                    writeln!(&mut wasfile,"{}. Local Set({}) {:?}", incount, loc, to_stack).expect("File Write Error");
                    call.vars[loc as usize] = to_stack;
                    //log::info!("Local Set: Index: {}, Value: {}", loc, to_stack);
                },
                Code::LocalTee(loc) =>
                {
                    let to_loc = self.value_stack.last().cloned().expect("Local Tee stk error");
                    let ind = loc as usize;
                    if ind >= call.vars.len()
                    {
                        panic!("LocalT: Index out of calls");
                    }
                    writeln!(&mut wasfile,"{}. LocalTee({}) {:?}", incount, loc, to_loc).expect("File Write Error");
                    call.vars[ind] = to_loc;

                },
                Code::GlobalGet(loc) =>
                {
                    let mut loc = loc as usize;
                    assert!(loc <= self.globs.len());
                    let to_stack = self.globs[loc as usize].typ.clone();
                    writeln!(&mut wasfile,"{}. Global Get({}) {:?}", incount, loc, to_stack).expect("File Write Error");
                    self.value_stack.push(to_stack);
                    //log::info!("Global Get: Index: {}, Value: {}", loc, to_stack);
                },
                Code::GlobalSet(loc) =>
                {
                    let to_glob = self.value_stack.pop().expect("Stack empty globset");
                    assert!(self.globs[loc as usize].ismut);
                    writeln!(&mut wasfile,"{}. Global Set({}) {:?}", incount, loc, to_glob).expect("File Write Error");
                    self.globs[loc as usize].typ = to_glob;
                    //log::info!("Global Set: Index: {}, Value: {}", loc, to_glob);
                },
                //Mem
                //LD
                Code::I32Load(off) =>
                {
                    let memloc = match self.value_stack.pop() {
                        Some(StackTypes::I32(loc)) => loc,
                        _ => panic!("Mem error"),
                    };
                    let offloc = off + memloc as u32;  
                    let of = offloc as usize;      
                    assert!(of + 4 <= self.mem.len());        
                    let bytes = &self.mem[of..of + 4];
                    let to_stack = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                    let val = StackTypes::I32(to_stack);
                    writeln!(&mut wasfile,"{}. I32Load({}) {}", incount, off, to_stack).expect("File Write Error");
                    self.value_stack.push(val);
                    //log::info!("I32 Load: Memomory Location: {}, Value: {}", memloc, val);
                },
                Code::I64Load(off) =>
                {
                    let memloc = match self.value_stack.pop() {
                        Some(StackTypes::I32(loc)) => loc,
                        _ => panic!("Mem error"),
                    };
                    let offloc = off + memloc as u32;  
                    let of = offloc as usize;              
                    let bytes = &self.mem[of..of + 8];
                    let to_stack = i64::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]]);
                    writeln!(&mut wasfile,"{}. I64Load({}) {}", incount, off, to_stack).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(to_stack));
                },
                Code::F32Load(off) => 
                {
                    let memloc = match self.value_stack.pop() {
                    Some(StackTypes::I32(loc)) => loc as u32,
                    _ => panic!("Mem error"),
                    };
                    let offloc = off + memloc;  
                    let of = offloc as usize;              
                    let bytes = &self.mem[of..of + 4];
                    let to_stack = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                    writeln!(&mut wasfile,"{}. F32Load({}) {}", incount, off, to_stack).expect("File Write Error");
                    self.value_stack.push(StackTypes::F32(to_stack));
                },
                Code::F64Load(off) =>
                {
                    let memloc = match self.value_stack.pop() {
                        Some(StackTypes::I32(loc)) => loc as u32,
                        _ => panic!("Mem error"),
                    };
                    let offloc = off + memloc;  
                    let of = offloc as usize;              
                    let bytes = &self.mem[of..of + 8];
                    let to_stack = f64::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]]);
                    writeln!(&mut wasfile,"{}. F64Load({}) {}", incount, off, to_stack).expect("File Write Error");
                    self.value_stack.push(StackTypes::F64(to_stack));
                },
                //I32
                Code::I32Load8S(off) => 
                {
                    let memloc = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(loc)) => loc as u32,
                        _ => panic!("Invalid stack type exp i32. I32Load8S"),
                    };
                    let offloc = (off + memloc) as usize;
                    let val = self.mem[offloc] as i8;
                    writeln!(&mut wasfile,"{}. I32Load8({}) {}", incount, off, val).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32(val as i32));
                },
                Code::I32Load8U(off) =>
                {
                    let memloc = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(loc)) => loc as u32,
                        _ => panic!("Invalid stack type exp i32. I32Load8U"),
                    };
                    let offloc = (off + memloc) as usize;
                    let val = self.mem[offloc] as u8;
                    writeln!(&mut wasfile,"{}. I32Load8U({}) {}", incount, off, val).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32(val as i32));

                },
                Code::I32Load16S(off) =>
                {
                    let memloc = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u32,
                        _ => panic!("Invalid stack type exp i32. I32Load16S"),
                    };
                    let offloc = (off + memloc) as usize;
                    let bytes = &self.mem[offloc..offloc + 2];
                    let val = i16::from_le_bytes([bytes[0], bytes[1]]);
                    writeln!(&mut wasfile,"{}. I32Load16S({}) {}", incount, off, val).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32(val as i32));
                },
                Code::I32Load16U(off) => 
                {
                    let memloc = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u32,
                        _ => panic!("Invalid stack type exp i32. I32Load16S"),
                    };
                    let offloc = (off + memloc) as usize;
                    let bytes = &self.mem[offloc..offloc + 2];
                    let val = u16::from_le_bytes([bytes[0], bytes[1]]);
                    writeln!(&mut wasfile,"{}. I32Load16U({}) {}", incount, off, val).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32(val as i32));
                },
                //I64
                Code::I64Load8S(off) =>
                {
                    let memloc = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u32,
                        _ => panic!("Invalid stack type exp i32. I64Load8S"),
                    };
                    let offloc = (off + memloc) as usize;
                    let val = self.mem[offloc] as i8 as i64;
                    writeln!(&mut wasfile,"{}. I64Load8S({}) {}", incount, off, val).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(val));
                },
                Code::I64Load8U(off) => 
                {
                    let memloc = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u32,
                        _ => panic!("Invalid stack type exp i32. I64Load8U"),
                    };
                    let offloc = (memloc + off) as usize;
                    let val = self.mem[offloc] as u8 as i64;
                    writeln!(&mut wasfile,"{}. I64Load8U({}) {}", incount, off, val).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(val));
                },
                Code::I64Load16S(off) =>
                {
                    let memloc = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u32,
                        _ => panic!("Invalid stack type exp i32. I64Load16S"),
                    };
                    let offloc = (off + memloc) as usize;
                    let bytes = &self.mem[offloc..offloc + 2];
                    let val = i16::from_le_bytes([bytes[0], bytes[1]]);
                    writeln!(&mut wasfile,"{}. I64Load16S({}) {}", incount, off, val).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(val as i64));
                },
                Code::I64Load16U(off) =>
                {
                    let memloc = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u32,
                        _ => panic!("Invalid stack type exp i32. I64Load16U"),
                    };
                    let offloc = (off + memloc) as usize;
                    let bytes = &self.mem[offloc..offloc + 2];
                    let val = u16::from_le_bytes([bytes[0], bytes[1]]);
                    writeln!(&mut wasfile,"{}. I64Load16U({}) {}", incount, off, val).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(val as i64));

                },
                Code::I64Load32S(off) =>
                {
                    let memloc = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u32,
                        _ => panic!("Invalid stack type exp i32. I64Load32S"),
                    };
                    let offloc = (off + memloc) as usize;
                    let bytes = &self.mem[offloc..offloc + 4];
                    let val = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                    writeln!(&mut wasfile,"{}. I64Load32S({}) {}", incount, off, val).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(val as i64));
                },
                Code::I64Load32U(off) =>
                {
                    let memloc = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u32,
                        _ => panic!("Invalid stack type exp i32. I64Load32U"),
                    };
                    let offloc = (off + memloc) as usize;
                    let bytes = &self.mem[offloc..offloc + 4];
                    let val = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                    writeln!(&mut wasfile,"{}. I64Load32U({}) {}", incount, off, val).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(val as i64));
                },
                //STR
                Code::I32Store(off) =>
                {
                    let var = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val,
                        _ => panic!("Store Stack err"),
                    };
                    let memloc = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u32,
                        _ => panic!("Store Stack Err"),
                    };
                    let uloc = (off + memloc) as usize;
                    let bytes = var.to_le_bytes();
                    writeln!(&mut wasfile,"{}. I32Store({}) {:?}", incount, off, bytes).expect("File Write Error");
                    self.mem[uloc..uloc + 4].copy_from_slice(&bytes);
                    //log::info!("I32 Store: Memory ");
                },
                Code::I64Store(off) =>
                {
                    let var = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(val)) => val,
                        _ => panic!("Store Stack err"),
                    };
                    let memloc = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u32,
                        _ => panic!("Store Stack Err"),
                    };
                    let uloc = (off + memloc) as usize;
                    let bytes = var.to_le_bytes();
                    writeln!(&mut wasfile,"{}. I64Store({}) {:?}", incount, off, bytes).expect("File Write Error");
                    self.mem[uloc..uloc + 8].copy_from_slice(&bytes);
                },
                Code::F32Store(off) =>
                {
                    let var = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(val)) => val,
                        _ => panic!("Store Stack err"),
                    };
                    let memloc = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u32,
                        _ => panic!("Store Stack Err"),
                    };
                    let uloc = (off + memloc) as usize;
                    let bytes = var.to_le_bytes();
                    writeln!(&mut wasfile,"{}. F32Store({}) {:?}", incount, off, bytes).expect("File Write Error");
                    self.mem[uloc..uloc + 4].copy_from_slice(&bytes);                    
                },
                Code::F64Store(off) =>
                {
                    let var = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(val)) => val,
                        _ => panic!("Store Stack err"),
                    };
                    let memloc = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u32,
                        _ => panic!("Store Stack Err"),
                    };
                    let uloc = (off + memloc) as usize;
                    let bytes = var.to_le_bytes();
                    writeln!(&mut wasfile,"{}. F64Store({}) {:?}", incount, off, bytes).expect("File Write Error");
                    self.mem[uloc..uloc + 8].copy_from_slice(&bytes);
                },
                Code::I32Store8(off) =>
                {
                    let var = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u8,
                        _ => panic!("Invalid stacktype I32Store8"), 
                    };
                    let memloc = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u32,
                        _ => panic!("Invalid stacktype I32Store8"),
                    };
                    let uloc = (off + memloc) as usize;
                    writeln!(&mut wasfile,"{}. I32Store8({}) {}", incount, off, var).expect("File Write Error");
                    self.mem[uloc] = var; 
                },
                Code::I32Store16(off) =>
                {
                    let var = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u16,
                        _ => panic!("Invalid stacktype I32Store8"), 
                    };
                    let memloc = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u32,
                        _ => panic!("Invalid stacktype I32Store8"),
                    };
                    let uloc = (off + memloc) as usize;
                    writeln!(&mut wasfile,"{}. I32Store16({}) {}", incount, off, var).expect("File Write Error");
                    self.mem[uloc..uloc + 2].copy_from_slice(&var.to_le_bytes()); 
                },
                Code::I64Store8(off) =>
                {
                    let var = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(val)) => val as u8,
                        _ => panic!("Invalid stacktype I32Store8"), 
                    };
                    let memloc = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u32,
                        _ => panic!("Invalid stacktype I32Store8"),
                    };
                    let uloc = (off + memloc) as usize;
                    writeln!(&mut wasfile,"{}. I64Store8({}) {}", incount, off, var).expect("File Write Error");
                    self.mem[uloc] = var; 
                },
                Code::I64Store16(off) =>
                {
                    let var = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(val)) => val as u16,
                        _ => panic!("Invalid stacktype I32Store8"), 
                    };
                    let memloc = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u32,
                        _ => panic!("Invalid stacktype I32Store8"),
                    };
                    let uloc = (off + memloc) as usize;
                    writeln!(&mut wasfile,"{}. I64Store16({}) {}", incount, off, var).expect("File Write Error");
                    self.mem[uloc..uloc + 2].copy_from_slice(&var.to_le_bytes()); 
                },
                Code::I64Store32(off) =>
                {
                    let var = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(val)) => val as u32,
                        _ => panic!("Invalid stacktype I32Store8"), 
                    };
                    let memloc = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u32,
                        _ => panic!("Invalid stacktype I32Store8"),
                    };
                    let uloc = (off + memloc) as usize;
                    writeln!(&mut wasfile,"{}. I64Store32({}) {}", incount, off, var).expect("File Write Error");
                    self.mem[uloc..uloc + 4].copy_from_slice(&var.to_le_bytes()); 
                },
                Code::MemorySize => 
                {
                    let memlen = self.mem.len();
                    writeln!(&mut wasfile,"{}. MemorySize {} ", incount, memlen).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32((memlen/65536) as i32));
                },
                Code::MemoryGrow => 
                {
                    let memchange = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(change)) => change,
                        _ => panic!("Invalid type memchange"),
                    };
                    assert!(memchange >= 0);
                    let curmem = (self.mem.len()/65536) as i32;
                    let newmem = ((curmem + memchange) * 65536) as u32;
                    if let Some(val) = self.memmax
                    {
                        assert!(val > newmem);
                    }
                    self.mem.resize(newmem as usize, 0);
                    writeln!(&mut wasfile,"{}. MemoryGrow New: {} Old: {}", incount, newmem, curmem).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32(curmem));

                },
                //Cons
                Code::I32Const(cons) => {
                    writeln!(&mut wasfile,"{}. I32Const {}", incount, cons).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32(cons));
                    //log::info!("I32 Constant: {}", cons);
                },
                Code::I64Const(cons) => {
                    writeln!(&mut wasfile,"{}. I64Const {}", incount, cons).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(cons));
                    //log::info!("I64 Constant: {}", cons);
                },
                Code::F32Const(cons) => {
                    writeln!(&mut wasfile,"{}. F32Const {}", incount, cons).expect("File Write Error");
                    self.value_stack.push(StackTypes::F32(cons));
                    //log::info!("F32 Constant: {}", cons);
                },
                Code::F64Const(cons) => {
                    writeln!(&mut wasfile,"{}. F64Const {}", incount, cons).expect("File Write Error");
                    self.value_stack.push(StackTypes::F64(cons));
                    //log::info!("F64 Constant {}", cons);
                },
                //Comps
                //I32
                Code::I32Eqz => {
                    let i_val = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. I32Eqz {}", incount, i_val).expect("File Write Error");
                    match i_val
                    {
                        0 => self.value_stack.push(StackTypes::I32(1)),
                        _ => self.value_stack.push(StackTypes::I32(0)),
                    }

                },
                Code::I32Eq => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(v2)) => v2,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(v1)) => v1,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. I32Eq Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 == val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I32Ne => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(v2)) => v2,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(v1)) => v1,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. I32Ne Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 != val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I32LtS => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(v2)) => v2,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(v1)) => v1,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. I32LtS Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 < val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I32LtU => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(v2)) => v2 as u32,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(v1)) => v1 as u32,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. I32LtU Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 < val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I32GtS => 
                {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(v2)) => v2,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(v1)) => v1,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. I32GtS Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 > val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I32GtU => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(v2)) => v2 as u32,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(v1)) => v1 as u32,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. I32GtU Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 > val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I32LeS => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(v2)) => v2,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(v1)) => v1,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. I32LeS Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 <= val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I32LeU => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(v2)) => v2 as u32,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(v1)) => v1 as u32,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. I32LeU Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 <= val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I32GeS => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(v2)) => v2,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(v1)) => v1,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. I32GeS Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 >= val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I32GeU => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(v2)) => v2 as u32,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(v1)) => v1 as u32,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. I32GeU Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 >= val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                //I64
                Code::I64Eqz => {
                    let val = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v)) => v,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. I64Eqz {}", incount, val).expect("File Write Error");
                    if val == 0 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I64Eq => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v2)) => v2,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v1)) => v1,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. I64Eq Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 == val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I64Ne => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v2)) => v2,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v1)) => v1,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. I64Ne Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 != val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I64LtS => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v2)) => v2 as u32,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v1)) => v1 as u32,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. I64LtS Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 < val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I64LtU => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v2)) => v2 as u32,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v1)) => v1 as u32,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. I64LtU Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 < val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I64GtS => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v2)) => v2,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v1)) => v1,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. I64GtS Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 > val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I64GtU => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v2)) => v2 as u32,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v1)) => v1 as u32,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. I64GtU Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 > val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I64LeS => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v2)) => v2,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v1)) => v1,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. I64LeS Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 <= val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I64LeU => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v2)) => v2 as u32,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v1)) => v1 as u32,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. I64LeU Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 <= val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I64GeS => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v2)) => v2,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v1)) => v1,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. I64GeS Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 >= val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I64GeU => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v2)) => v2 as u32,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v1)) => v1 as u32,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. I64GeU Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 >= val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                //F32
                Code::F32Eq => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(v2)) => v2,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(v1)) => v1,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. F32Eq Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 == val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::F32Ne => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(v2)) => v2,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(v1)) => v1,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. F32Ne Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 != val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::F32Lt => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(v2)) => v2,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(v1)) => v1,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. F32Lt Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 < val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::F32Gt => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(v2)) => v2,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(v1)) => v1,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. F32Gt Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 > val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::F32Le => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(v2)) => v2,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(v1)) => v1,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. F32Le Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 <= val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                   //Calcs
                //I32
//                Code::I32Clz => (),
                Code::I32Clz => {
                    let val = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(v)) => v,
                        _ => panic!("Invalid type stack error"),
                    };
                    let leading_zeros = val.leading_zeros();
                    writeln!(&mut wasfile,"{}. I32Clz {}", incount, val).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32(leading_zeros as i32));
                },
//               Code::I32Ctz => (),
                Code::I32Ctz => {
                    let val = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(v)) => v,
                        _ => panic!("Invalid type stack error"),
                    };
                    let trailing_zeros = val.trailing_zeros();
                    writeln!(&mut wasfile,"{}. I32Ctz {}", incount, val).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32(trailing_zeros as i32));
                },  
//                Code::I32Popcnt => (),
                Code::I32Popcnt => {
                    let val = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(v)) => v,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. I32Popcnt {}", incount, val).expect("File Write Error");
                    let popcnt = val.count_ones();
                    self.value_stack.push(StackTypes::I32(popcnt as i32));
                },  
                Code::I32Add => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val,
                        _ => panic!("Add error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val,
                        _ => panic!("Add error"),
                    };
                    writeln!(&mut wasfile,"{}. I32Add Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32(val1+val2));
                    //log::info!("I32 Add: {} + {}", y, x);
                },
                Code::I32Sub => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val,
                        _ => panic!("Sub error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val,
                        _ => panic!("Sub error"),
                    };
                    writeln!(&mut wasfile,"{}. I32Sub Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32(val1-val2));
                    //log::info!("I32 Subtract: {} - {}", y, x);
                },
                Code::I32Mul => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val,
                        _ => panic!("Mul error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val,
                        _ => panic!("Mul error"),
                    };
                    writeln!(&mut wasfile,"{}. I32Mul Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32(val1*val2));
                    //log::info!("I32 Multiplication: {} * {}", y, x);
                },
//                Code::I32DivS => (),
                    Code::I32DivS => {
                    let val2 = match self.value_stack.pop() {
                        Some(StackTypes::I32(v)) => v,
                        _ => panic! ("I32Divs error"),
                    };
                    let val1 = match self.value_stack.pop() {
                        Some(StackTypes::I32(v)) => v,
                        _ => panic! ("I32Divs error"),
                    };
                    writeln!(&mut wasfile,"{}. I32DivS Val1: {}/ Val2: {}", incount, val1, val2).expect("File Write Error");
                   self.value_stack.push(StackTypes::I32(val1 / val2));
                },
//                Code::I32DivU => (),
                Code::I32DivU => {
                    let val2 = match self.value_stack.pop() {
                    Some(StackTypes::I32(v)) => v as u32,
                    _ => panic! ("I32Divu error"),
                    };
                let val1 = match self.value_stack.pop() {
                    Some(StackTypes::I32(v)) => v as u32,
                    _ => panic! ("I32Divu error"),
                };
                writeln!(&mut wasfile,"{}. I32DivU Val1: {}/ Val2: {}", incount, val1, val2).expect("File Write Error");
                self.value_stack.push(StackTypes::I32((val1 / val2) as i32));
                },
                //                Code::I32RemS => (),
                Code::I32RemS => {
                    let b = match self.value_stack.pop() {
                        Some(StackTypes::I32(v)) => v,
                        _ => panic! ("I32Rems error"),
                    };
                    let a = match self.value_stack.pop() {
                        Some(StackTypes::I32(v)) => v,
                        _ => panic! ("I32Rems error"),
                    };
                    writeln!(&mut wasfile,"{}. I32RemS Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                self.value_stack.push(StackTypes::I32(a % b));
                },

                //                Code::I32RemU => (),
                Code::I32RemU => {
                    let b = match self.value_stack.pop() {
                        Some(StackTypes::I32(v)) => v as u32,
                        _ => panic! ("I32Remu error"),
                    };
                    let a = match self.value_stack.pop() {
                        Some(StackTypes::I32(v)) => v as u32,
                        _ => panic! ("I32Remu error"),
                    };
                    writeln!(&mut wasfile,"{}. I32RemU Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32((a % b) as i32));
                },
                //                Code::I32And => (),
                Code::I32And => {
                    let b = match self.value_stack.pop() {
                        Some(StackTypes::I32(v)) => v,
                        _ => panic!("I32And error"),
                    };
                    let a = match self.value_stack.pop() {
                        Some(StackTypes::I32(v)) => v,
                        _ => panic!("I32And error"),
                    };
                    writeln!(&mut wasfile,"{}. I32And Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32(a & b));
                },
                //                Code::I32Or => (),
                Code::I32Or => {
                    let b = match self.value_stack.pop() {
                        Some(StackTypes::I32(v)) => v,
                        _ => panic!("I32Or error"),
                    };
                    let a = match self.value_stack.pop() {
                        Some(StackTypes::I32(v)) => v,
                        _ => panic!("I32Or error"),
                    };
                    writeln!(&mut wasfile,"{}. I32Or Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32(a | b));
                },
                //                Code::I32Xor => (),
                Code::I32Xor => {
                    let b = match self.value_stack.pop() {
                        Some(StackTypes::I32(v)) => v,
                        _ => panic!("I32Xor error"),
                    };
                    let a = match self.value_stack.pop() {
                        Some(StackTypes::I32(v)) => v,
                        _ => panic!("I32Xor error"),
                    };
                    writeln!(&mut wasfile,"{}. I32Xor Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32(a ^ b));
                },
                //                Code::I32Shl => (),
                Code::I32Shl => {
                    let b = match self.value_stack.pop() {
                        Some(StackTypes::I32(v)) => v as u32,
                        _ => panic!("I32Shl error"),
                    };
                    let a = match self.value_stack.pop() {
                        Some(StackTypes::I32(v)) => v,
                        _ => panic!("I32Shl error"),
                    };
                    writeln!(&mut wasfile,"{}. I32Shl Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32(a << b));
                },
                //                Code::I32ShrS => (),
                Code::I32ShrS => {
                    let b = match self.value_stack.pop() {
                        Some(StackTypes::I32(v)) => v,
                        _ => panic!("I32ShrS error"),
                    };
                    let a = match self.value_stack.pop() {
                        Some(StackTypes::I32(v)) => v,
                        _ => panic!("I32ShrS error"),
                    };
                    writeln!(&mut wasfile,"{}. I32ShrS Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32(a >> b));
                },
                //                Code::I32ShrU => (),
                Code::I32ShrU => {
                    let b = match self.value_stack.pop() {
                        Some(StackTypes::I32(v)) => v as u32,
                        _ => panic!("I32ShrU error"),
                    };
                    let a = match self.value_stack.pop() {
                        Some(StackTypes::I32(v)) => v as u32,
                        _ => panic!("I32ShrU error"),
                    };
                    writeln!(&mut wasfile,"{}. I32ShrU Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32((a >> b) as i32));
                },
                //                Code::I32Rotl => (),
                Code::I32Rotl => {
                    let shift = match self.value_stack.pop() {
                        Some(StackTypes::I32(v)) => v as u32,
                        _ => panic!("I32Rotl error"),
                    };
                    let value = match self.value_stack.pop() {
                        Some(StackTypes::I32(v)) => v,
                        _ => panic!("I32Rotl error"),
                    };
                    writeln!(&mut wasfile,"{}. I32Rotl Shift: {} Val: {}", incount, shift, value).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32(value.rotate_left(shift) as i32));
                },
                //                Code::I32Rotr => (),
                Code::I32Rotr => {
                    let shift = match self.value_stack.pop() {
                        Some(StackTypes::I32(v)) => v as u32,
                        _ => panic!("I32Rotr error"),
                    };
                    let value = match self.value_stack.pop() {
                        Some(StackTypes::I32(v)) => v,
                        _ => panic!("I32Rotr error"),
                    };
                    writeln!(&mut wasfile,"{}. I32Rotr Shift: {} Value: {}", incount, shift, value).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32(value.rotate_right(shift) as i32));
                },
                                //I64
                //                Code::I64Clz => (),
                Code::I64Clz => {
                    let value = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v,
                        _ => panic!("I64Clz error"),
                    };
                    writeln!(&mut wasfile,"{}. I64Clz Value: {}", incount, value).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(value.leading_zeros() as i64));
                },
                //                Code::I64Ctz => (),
                Code::I64Ctz => {
                    let value = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v,
                        _ => panic!("I64Ctz error"),
                    };
                    writeln!(&mut wasfile,"{}. I64Ctz Value: {}", incount, value).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(value.trailing_zeros() as i64));
                },
                //                Code::I64Popcnt => (),
                Code::I64Popcnt => {
                    let value = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v,
                        _ => panic!("I64Popcnt error"),
                    };
                    writeln!(&mut wasfile,"{}. I64Popcnt Value: {}", incount, value).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(value.count_ones() as i64));
                },
                //                Code::I64Add => (),
                Code::I64Add => {
                    let b = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(val)) => val,
                        _ => panic!("Add error"),
                    };
                    let a = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(val)) => val,
                        _ => panic!("Add error"),
                    };
                    writeln!(&mut wasfile,"{}. I64Add Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(a+b));
                },
                //                Code::I64Sub => (),
                Code::I64Sub => {
                    let b = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(val)) => val,
                        _ => panic!("Sub error"),
                    };
                    let a = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(val)) => val,
                        _ => panic!("Sub error"),
                    };
                    writeln!(&mut wasfile,"{}. I64Sub Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(a-b));
                },
                //                Code::I64Mul => (),
                Code::I64Mul => {
                    let b = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(val)) => val,
                        _ => panic!("Mul error"),
                    };
                    let a = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(val)) => val,
                        _ => panic!("Mul error"),
                    };    
                    writeln!(&mut wasfile,"{}. I64Mul Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(a*b));
                },
                //                Code::I64DivS => (),
                Code::I64DivS => {
                    let b = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v,
                        _ => panic! ("I64Divs error"),
                    };
                    let a = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v,
                        _ => panic! ("I64Divs error"),
                    };
                    writeln!(&mut wasfile,"{}. I64DivS Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(a / b));
                },
                //                Code::I64DivU => (),
                Code::I64DivU => {
                    let b = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v as u64,
                        _ => panic! ("I64Divu error"),
                    };
                    let a = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v as u64,
                        _ => panic! ("I64Divu error"),
                    };
                    writeln!(&mut wasfile,"{}. I64DivU Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64((a / b) as i64));
                },
                //                Code::I64RemS => (),
                Code::I64RemS => {
                    let b = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v,
                        _ => panic! ("I64Rems error"),
                    };
                    let a = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v,
                        _ => panic! ("I64Rems error"),
                    };
                    writeln!(&mut wasfile,"{}. I64RemS Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(a % b));
                },
                //                Code::I64RemU => (),
                Code::I64RemU => {
                    let b = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v as u64,
                        _ => panic! ("I64Remu error"),
                    };
                    let a = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v as u64,
                        _ => panic! ("I64Remu error"),
                    };
                    writeln!(&mut wasfile,"{}. I64RemU Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64((a % b) as i64));
                },
                //                Code::I64And => (),
                Code::I64And => {
                    let b = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v,
                        _ => panic!("I64And error"),
                    };
                    let a = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v,
                        _ => panic!("I64And error"),
                    };
                    writeln!(&mut wasfile,"{}. I64And Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(a & b));
                },
                //                Code::I64Or => (),
                Code::I64Or => {
                    let b = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v,
                        _ => panic!("I64Or error"),
                    };
                    let a = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v,
                        _ => panic!("I64Or error"),
                    };
                    writeln!(&mut wasfile,"{}. I64Or Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(a | b));
                },
                //                Code::I64Xor => (),
                Code::I64Xor => {
                    let b = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v,
                        _ => panic!("I64Xor error"),
                    };
                    let a = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v,
                        _ => panic!("I64Xor error"),
                    };
                    writeln!(&mut wasfile,"{}. I64Xor Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(a ^ b));
                },
                //                Code::I64Shl => (),
                Code::I64Shl => {
                    let shift = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v as u64,
                        _ => panic!("I64Shl error"),
                    };
                    let value = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v,
                        _ => panic!("I64Shl error"),
                    };
                    writeln!(&mut wasfile,"{}. I64Shl Shift: {} Val: {}", incount, shift, value).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(value << shift));
                },
                //                Code::I64ShrS => (),
                Code::I64ShrS => {
                    let shift = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v as u64,
                        _ => panic!("I64ShrS error"),
                    };
                    let value = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v,
                        _ => panic!("I64ShrS error"),
                    };
                    writeln!(&mut wasfile,"{}. I64ShrS Shift: {} Val: {}", incount, shift, value).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(value >> shift));
                },
                //                Code::I64ShrU => (),
                Code::I64ShrU => {
                    let shift = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v as u64,
                        _ => panic!("I64ShrU error"),
                    };
                    let value = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v as u64,
                        _ => panic!("I64ShrU error"),
                    };
                    writeln!(&mut wasfile,"{}. I64 Shift: {} Value: {}", incount, shift, value).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64((value >> shift) as i64));
                },
                //                Code::I64Rotl => (),
                Code::I64Rotl => {
                    let shift = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v as u32,
                        _ => panic!("I64Rotl error"),
                    };
                    let value = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v as u64,
                        _ => panic!("I64Rotl error"),
                    };
                    writeln!(&mut wasfile,"{}. I64Rotl Shift: {} value: {}", incount, shift, value).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(value.rotate_left(shift) as i64));
                },
                //                Code::I64Rotr => (),
                Code::I64Rotr => {
                    let shift = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v as u32,
                        _ => panic!("I64Rotr error"),
                    };
                    let value = match self.value_stack.pop() {
                        Some(StackTypes::I64(v)) => v as u64,
                        _ => panic!("I64Rotr error"),
                    };
                    writeln!(&mut wasfile,"{}. I64Rotr Shift: {} Value: {}", incount, shift, value).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(value.rotate_right(shift) as i64));
                },
                                //FL
                                //F32
                //                Code::F32Abs => (),
                Code::F32Abs => {
                    let value = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(v)) => v,
                        _ => panic!("F32Abs error"),
                    };
                    writeln!(&mut wasfile,"{}. F32Abs Val: {}", incount, value).expect("File Write Error");
                    self.value_stack.push(StackTypes::F32(value.abs()));
                },
                //                Code::F32Neg => (),
                Code::F32Neg => {
                    let value = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(v)) => v,
                        _ => panic!("F32Neg error"),
                    };
                    writeln!(&mut wasfile,"{}. F32Neg Val: {}", incount, value).expect("File Write Error");
                    self.value_stack.push(StackTypes::F32(-value));
                },
                //                Code::F32Ceil => (),
                Code::F32Ceil => {
                    let value = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(v)) => v,
                        _ => panic!("F32Ceil error"),
                    };
                    writeln!(&mut wasfile,"{}. F32Ceil Val: {}", incount, value).expect("File Write Error");
                    self.value_stack.push(StackTypes::F32(value.ceil()));
                },
                //                Code::F32Floor => (),
                Code::F32Floor => {
                    let value = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(v)) => v,
                        _ => panic!("F32Floor error"),
                    };
                    writeln!(&mut wasfile,"{}. F32Floor Val: {}", incount, value).expect("File Write Error");
                    self.value_stack.push(StackTypes::F32(value.floor()));
                },
                //                Code::F32Trunc => (),
                Code::F32Trunc => {
                    let value = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(v)) => v,
                        _ => panic!("F32Trunc error"),
                    };
                    writeln!(&mut wasfile,"{}. F32Trunc Val: {}", incount, value).expect("File Write Error");
                    self.value_stack.push(StackTypes::F32(value.trunc()));
                },
                //                Code::F32Nearest => (),
                Code::F32Nearest => {
                    let value = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(v)) => v,
                        _ => panic!("F32Nearest error"),
                    };
                    writeln!(&mut wasfile,"{}. F32Nearest Val: {}", incount, value).expect("File Write Error");
                    self.value_stack.push(StackTypes::F32(value.round()));
                },
                //                Code::F32Sqrt => (),
                Code::F32Sqrt => {
                    let value = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(v)) => v,
                        _ => panic!("F32Sqrt error"),
                    };
                    writeln!(&mut wasfile,"{}. F32Sqrt Val: {}", incount, value).expect("File Write Error");
                    self.value_stack.push(StackTypes::F32(value.sqrt()));
                },
                //                Code::F32Add => (),
                Code::F32Add => {
                    let b = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(val)) => val,
                        _ => panic!("F32Add error"),
                    };
                    let a = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(val)) => val,
                        _ => panic!("F32Add error"),
                    };
                    writeln!(&mut wasfile,"{}. F32Add Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::F32(a+b));
                },
                //                Code::F32Sub => (),
                Code::F32Sub => {
                    let b = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(val)) => val,
                        _ => panic!("F32Sub error"),
                    };
                    let a = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(val)) => val,
                        _ => panic!("F32Sub error"),
                    };
                    writeln!(&mut wasfile,"{}. F32Sub Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::F32(a-b));
                },
                //                Code::F32Mul => (),
                Code::F32Mul => {
                    let b = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(val)) => val,
                        _ => panic!("F32Mul error"),
                    };
                    let a = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(val)) => val,
                        _ => panic!("F32Mul error"),
                    };
                    writeln!(&mut wasfile,"{}. F32Mul Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::F32(a*b));
                },
                //                Code::F32Div => (),
                Code::F32Div => {
                    let b = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(val)) => val,
                        _ => panic!("F32Div error"),
                    };
                    let a = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(val)) => val,
                        _ => panic!("F32Div error"),
                    };
                    writeln!(&mut wasfile,"{}. F32Div Val1: {}/ Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::F32(a/b));
                },
                //                Code::F32Min => (),
                Code::F32Min => {
                    let b = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(val)) => val,
                        _ => panic!("F32Min error"),
                    };
                    let a = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(val)) => val,
                        _ => panic!("F32Min error"),
                    };
                    writeln!(&mut wasfile,"{}. F32Min Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::F32(a.min(b)));
                },
                //                Code::F32Max => (),
                Code::F32Max => {
                    let b = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(val)) => val,
                        _ => panic!("F32Max error"),
                    };
                    let a = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(val)) => val,
                        _ => panic!("F32Max error"),
                    };
                    writeln!(&mut wasfile,"{}. F32Max Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::F32(a.max(b)));
                },
                //                Code::F32Copysign => (),
                Code::F32Copysign => {
                    let sign = match self.value_stack.pop(){
                        Some(StackTypes::F32(v)) => v,
                        _ => panic!("F32Copysign error"),
                    };
                    let value = match self.value_stack.pop(){
                        Some(StackTypes::F32(v)) => v,
                        _ => panic!("F32Copysign error"),
                    };
                    writeln!(&mut wasfile,"{}. F32Copysign Sign: {} Value: {}", incount, sign, value).expect("File Write Error");
                    self.value_stack.push(StackTypes::F32(sign.copysign(value)));
                },
                                //F64
                //                Code::F64Abs => (),
                Code::F64Abs => {
                    let v = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(v)) => v,
                        _ => panic!("F64Abs error"),
                    };
                    writeln!(&mut wasfile,"{}. F64Abs Value: {}", incount, v).expect("File Write Error");
                    self.value_stack.push(StackTypes::F64(v.abs()));
                },
                //                Code::F64Neg => (),
                Code::F64Neg => {
                    let v = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(v)) => v,
                        _ => panic!("F64Neg error"),
                    };
                    writeln!(&mut wasfile,"{}. F64Neg Value: {}", incount, v).expect("File Write Error");
                    self.value_stack.push(StackTypes::F64(-v));
                },
                //                Code::F64Ceil => (),
                Code::F64Ceil => {
                    let v = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(v)) => v,
                        _ => panic!("F64Ceil error"),
                    };
                    writeln!(&mut wasfile,"{}. F64Ceil Value: {}", incount, v).expect("File Write Error");
                    self.value_stack.push(StackTypes::F64(v.ceil()));
                },
                //                Code::F64Floor => (),
                Code::F64Floor => {
                    let v = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(v)) => v,
                        _ => panic!("F64Floor error"),
                    };
                    writeln!(&mut wasfile,"{}. F64Floor Value: {}", incount, v).expect("File Write Error");
                    self.value_stack.push(StackTypes::F64(v.floor()));
                },
                //                Code::F64Trunc => (),
                Code::F64Trunc => {
                    let v = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(v)) => v,
                        _ => panic!("F64Trunc error"),
                    };
                    writeln!(&mut wasfile,"{}. F64Trunc Value: {}", incount, v).expect("File Write Error");
                    self.value_stack.push(StackTypes::F64(v.trunc()));
                },
                //                Code::F64Nearest => (),
                Code::F64Nearest => {
                    let v = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(v)) => v,
                        _ => panic!("F64Nearest error"),
                    };
                    writeln!(&mut wasfile,"{}. F64Nearest Value: {}", incount, v).expect("File Write Error");
                    self.value_stack.push(StackTypes::F64(v.round()));
                },
                //                Code::F64Sqrt => (),
                Code::F64Sqrt => {
                    let v = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(v)) => v,
                        _ => panic!("F64Sqrt error"),
                    };
                    writeln!(&mut wasfile,"{}. F64Sqrt Value: {}", incount, v).expect("File Write Error");
                    self.value_stack.push(StackTypes::F64(v.sqrt()));
                },
                //                Code::F64Add => (),
                Code::F64Add => {
                    let b = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(val)) => val,
                        _ => panic!("F64Add error"),
                    };
                    let a = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(val)) => val,
                        _ => panic!("F64Add error"),
                    };
                    writeln!(&mut wasfile,"{}. F64Add Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::F64(a+b));
                },
                //                Code::F64Sub => (),
                Code::F64Sub => {
                    let b = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(val)) => val,
                        _ => panic!("F64Sub error"),
                    };
                    let a = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(val)) => val,
                        _ => panic!("F64Sub error"),
                    };
                    writeln!(&mut wasfile,"{}. F64Sub Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::F64(a-b));
                },
                //                Code::F64Mul => (),
                Code::F64Mul => {
                    let b = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(val)) => val,
                        _ => panic!("F64Mul error"),
                    };
                    let a = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(val)) => val,
                        _ => panic!("F64Mul error"),
                    };
                    writeln!(&mut wasfile,"{}. F64Mul Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::F64(a*b));
                },
                //                Code::F64Div => (),
                Code::F64Div => {
                    let b = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(val)) => val,
                        _ => panic!("F64Div error"),
                    };
                    let a = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(val)) => val,
                        _ => panic!("F64Div error"),
                    };
                    writeln!(&mut wasfile,"{}. F64Div Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::F64(a/b));
                },
                //                Code::F64Min => (),
                Code::F64Min => {
                    let b = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(val)) => val,
                        _ => panic!("F64Min error"),
                    };
                    let a = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(val)) => val,
                        _ => panic!("F64Min error"),
                    };
                    writeln!(&mut wasfile,"{}. F64Min Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::F64(a.min(b)));
                },
                //                Code::F64Max => (),
                Code::F64Max => {
                    let b = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(val)) => val,
                        _ => panic!("F64Max error"),
                    };
                    let a = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(val)) => val,
                        _ => panic!("F64Max error"),
                    };
                    writeln!(&mut wasfile,"{}. F64Max Val1: {} Val2: {}", incount, a, b).expect("File Write Error");
                    self.value_stack.push(StackTypes::F64(a.max(b)));
                },
                //                Code::F64Copysign => (),
                Code::F64Copysign => {
                    let sign = match self.value_stack.pop(){
                        Some(StackTypes::F64(v)) => v,
                        _ => panic!("F64Copysign error"),
                    };
                    let value = match self.value_stack.pop(){
                        Some(StackTypes::F64(v)) => v,
                        _ => panic!("F64Copysign error"),
                    };
                    writeln!(&mut wasfile,"{}. F64Copysign Sign: {} Value: {}", incount, sign, value).expect("File Write Error");
                    self.value_stack.push(StackTypes::F64(sign.copysign(value)));
                },
                //tools
                Code::F32Ge => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(v2)) => v2,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(v1)) => v1,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. F64Ge Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 >= val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                //F64
                Code::F64Eq => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(v2)) => v2,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(v1)) => v1,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. F64eq Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 == val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::F64Ne => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(v2)) => v2,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(v1)) => v1,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. F64Ne Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 != val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::F64Lt => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(v2)) => v2,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(v1)) => v1,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. F64Lt Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 < val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::F64Gt => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(v2)) => v2,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(v1)) => v1,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. F64Gt Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 > val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::F64Le => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(v2)) => v2,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(v1)) => v1,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. F64Le Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 <= val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::F64Ge => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(v2)) => v2,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(v1)) => v1,
                        _ => panic!("Invalid type stack error"),
                    };
                    writeln!(&mut wasfile,"{}. F64Ge Val1: {} Val2: {}", incount, val1, val2).expect("File Write Error");
                    if val1 >= val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                //tools
                Code::I32WrapI64 => 
                {
                    let wrapped = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(val)) => val as i32,
                        _ => panic!("Invalid Stack Type I32WrapI64"),
                    };
                    writeln!(&mut wasfile,"{}. I32WrapI64 Value: {}", incount, wrapped).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32(wrapped));
                },
                Code::I32TruncF32S => 
                {
                    let trunced = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(val)) => val as i32,
                        _ => panic!("Invalid Stack Type I32WrapF32S"),
                    };
                    writeln!(&mut wasfile,"{}. I32TruncF32S Value: {}", incount, trunced).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32(trunced))
                },
                Code::I32TruncF32U => 
                {
                    let trunced = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(val)) => {
                            if val < 0.0
                            {
                                panic!("Floating point number is less than 0 I32TruncF32u");
                            }
                            val as u32
                        },
                        _ => panic!("Invalid Stack Type I32TruncF32U"),
                    };
                    let sender = trunced as i32;
                    writeln!(&mut wasfile,"{}. I32TruncF32U Value: {}", incount, trunced).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32(sender));
                },
                Code::I32TruncF64S => 
                {
                    let trunced = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(val)) => val as i32,
                        _ => panic!("Stack type is not a F64 I32TruncF64S"),
                    };
                    writeln!(&mut wasfile,"{}. I32TruncF64S Value: {}", incount, trunced).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32(trunced));
                },
                Code::I32TruncF64U => 
                {
                    let trunced = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(val)) =>
                        {
                            if val < 0.0
                            {
                                panic!("Float is less than 0 I32TruncF64U");
                            }
                            val as u32
                        },
                        _ => panic!("Stack type is not a F64 I32TruncF64U"),
                    };
                    writeln!(&mut wasfile,"{}. I32TruncF64U Value: {}", incount, trunced).expect("File Write Error");
                    self.value_stack.push(StackTypes::I32(trunced as i32));
                },
                Code::I64ExtendI32S => 
                {
                    let extend = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as i64,
                        _ => panic!("Stack type is not I32 I64extendI32S"),
                    };
                    writeln!(&mut wasfile,"{}. I64ExtendI32S Value: {}", incount, extend).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(extend));
                },  
                Code::I64ExtendI32U => 
                {
                    let extend = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) =>
                        {
                            if val < 0
                            {
                                panic!("I32 Value is less than 0 I64ExtendI32U");
                            }
                            val as u64
                        },
                        _ => panic!("Stack type is not I32 I64ExtendI32U"),
                    };
                    writeln!(&mut wasfile,"{}. I64ExtendI32U Value: {}", incount, extend).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(extend as i64));
                },
                Code::I64TruncF32S => 
                {
                    let trunced = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(val)) => val as i64,
                        _ => panic!("Stack type is not F32 I64TruncF32S"),
                    };
                    writeln!(&mut wasfile,"{}. I64TruncF32S Value: {}", incount, trunced).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(trunced));
                },
                Code::I64TruncF32U => 
                {
                    let trunced = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(val)) =>
                        {
                            if val < 0.0
                            {
                                panic!("F32 Value is less than 0 I64TruncF32U");
                            }
                            val as u32
                        },
                        _ => panic!("Stack type is not F32 I64TruncF32U"),
                    };
                    writeln!(&mut wasfile,"{}. I64TruncF32U Value: {}", incount, trunced).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(trunced as i64));
                },
                Code::I64TruncF64S => 
                {
                    let trunced = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(val)) => val as i64,
                        _ => panic!("Stack type is not F64 I64TruncF64S"),
                    };
                    writeln!(&mut wasfile,"{}. I64TruncF64S Value: {}", incount, trunced).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(trunced));
                },
                Code::I64TruncF64U => 
                {
                    let trunced = match self.value_stack.pop()
                    {  
                        Some(StackTypes::F64(val)) =>
                        {
                            if val < 0.0
                            {
                                panic!("Floating point value less than zero I64TruncF64U");
                            }
                            val as u64
                        },
                        _ => panic!("Stack type is not F64 I64TruncF64U"),
                    };
                    writeln!(&mut wasfile,"{}. I64TruncF64U Value: {}", incount, trunced).expect("File Write Error");
                    self.value_stack.push(StackTypes::I64(trunced as i64));
                },
                Code::F32ConvertI32S => 
                {
                    let converted = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as f32,
                        _ => panic!("Stack type is not I32 F32ConvertI32S"),
                    };
                    writeln!(&mut wasfile,"{}. F32ConvertI32S Value: {}", incount, converted).expect("File Write Error");
                    self.value_stack.push(StackTypes::F32(converted));
                },
                Code::F32ConvertI32U => 
                {
                    let converted = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => 
                        {
                            if val < 0
                            {
                                panic!("I32 value less than zero F32ConvertI32U");
                            }
                            val as f32
                        },
                        _=> panic!("Stack type not I32 F32ConvertI32U"),
                    };
                    writeln!(&mut wasfile,"{}. F32ConvertI32U Value: {}", incount, converted).expect("File Write Error");
                    self.value_stack.push(StackTypes::F32(converted));
                },
                Code::F32ConvertI64S => 
                {
                    let converted = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(val)) => val as f32,
                        _ => panic!("Stack type not I64 F32ConvertI64S"),
                    };
                    writeln!(&mut wasfile,"{}. F32ConvertI64S Value: {}", incount, converted).expect("File Write Error");
                    self.value_stack.push(StackTypes::F32(converted));
                },
                Code::F32ConvertI64U => 
                {
                    let converted = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(val)) =>
                        {
                            if val < 0 {panic!("I64 value less than zero F32ConvertI64U");}
                            val as f32
                        },
                        _ => panic!("Stack type not I64 F32ConvertI64U"),
                    };
                    writeln!(&mut wasfile,"{}. F32ConvertI64U Value: {}", incount, converted).expect("File Write Error");
                    self.value_stack.push(StackTypes::F32(converted));
                },
                Code::F32DemoteF64 => 
                {
                    match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(val)) => {
                            writeln!(&mut wasfile,"{}. F32DemoteF64 Value: {}", incount, val).expect("File Write Error");
                            self.value_stack.push(StackTypes::F32(val as f32));
                        }
                        _ => panic!("Stack type not F64 F32DemoteF64"),
                    }
                },
                Code::F64ConvertI32S => 
                {
                    match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => {
                            writeln!(&mut wasfile,"{}. F64ConvertI32S Value: {}", incount, val).expect("File Write Error");
                            self.value_stack.push(StackTypes::F64(val as f64));                        
                        }
                        _ => panic!("Stack type not I32 F64ConvertI32"),
                    }
                },
                Code::F64ConvertI32U => 
                {
                    match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) =>
                        {
                            if val < 0 {panic!("I32 value is less than 0 F64ConvertI32U");}
                            writeln!(&mut wasfile,"{}. ConvertI32U Value: {}", incount, val).expect("File Write Error");
                            self.value_stack.push(StackTypes::F64(val as f64));
                        }
                        _ => panic!("Stack type not I32 F64ConvertI32U"),
                    }
                },
                Code::F64ConvertI64S => 
                {
                    match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(val)) => {
                            writeln!(&mut wasfile,"{}. F64ConvertI64S Value: {}", incount, val).expect("File Write Error");
                            self.value_stack.push(StackTypes::F64(val as f64));
                        }
                        _ => panic!("Stack type not I64 F64ConvertI64S"),
                    }
                },
                Code::F64ConvertI64U => 
                {
                    match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(val)) =>
                        {
                            if val < 0 {panic!("I64 value less than zero F64ConvertI64U");}
                            writeln!(&mut wasfile,"{}. F64ConvertI64U Value: {}", incount, val).expect("File Write Error");
                            self.value_stack.push(StackTypes::F64(val as f64));
                        },
                        _ => panic!("Stack type not I64 F64ConvertI64U"),
                    }
                },
                Code::F64PromoteF32 => 
                {
                    match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(val)) => {
                            writeln!(&mut wasfile,"{}. F64PromoteF32 Value: {}", incount, val).expect("File Write Error");
                            self.value_stack.push(StackTypes::F64(val as f64));
                        }
                        _ => panic!("Stack type not I32 F64PromoteF32"),
                    }
                },
                Code::I64ReinterpretF64 => 
                {
                    match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(val)) => {
                            writeln!(&mut wasfile,"{}. I64ReinterpretF64 Value: {}", incount, val).expect("File Write Error");
                            self.value_stack.push(StackTypes::I64(f64::to_bits(val) as i64));
                        }
                        _ => panic!("Stack type not F64 I64ReinterpretF64"),
                    }
                },
                Code::I32ReinterpretF32 => 
                {
                    match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(val)) => {
                            writeln!(&mut wasfile,"{}. I32ReinterpretF32 Value: {}", incount, val).expect("File Write Error");
                            self.value_stack.push(StackTypes::I32(val.to_bits() as i32));
                        }
                        _ => panic!("Stack type not F32 I32 ReinterpretF32"),
                    }
                },
                Code::F64ReinterpretI64 => 
                {
                    match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(val)) => {
                            writeln!(&mut wasfile,"{}. F64ReinterpretI64 Value: {}", incount, val).expect("File Write Error");
                            self.value_stack.push(StackTypes::F64(f64::from_bits(val as u64)));
                        }
                        _ => panic!("Stack type not I64 F64ReinterpretI64"),
                    }
                },
                Code::F32ReinterpretI32 => 
                {
                    match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => {
                            writeln!(&mut wasfile,"{}. F32ReinterpretI32 Value: {}", incount, val).expect("File Write Error");
                            self.value_stack.push(StackTypes::F32(f32::from_bits(val as u32)));
                        }
                        _ => panic!("Stack type not I32 F32ReinterpretI32"),
                    }
                },
                _ => panic!("Unsupported Type"),
            }
            incount += 1;
        }
        //wasfile.flush().expect("Cant flush log file");
    }
}
