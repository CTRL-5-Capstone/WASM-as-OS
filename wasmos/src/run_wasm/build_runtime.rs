use core::panic;

use super::wasm_module::*;
#[derive(Clone)]
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
pub struct Runtime
{
    pub module: Module,
    pub mem: Vec<u8>,
    pub call_stack: Vec<StackCalls>, 
    pub value_stack: Vec<StackTypes>,
    pub flow_stack: Vec<FlowCode>,
    pub globs: Vec<StackTypes>,

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
        let to_mem = module.memy.first().map(|(min, _max)| *min).unwrap_or(1);
        let mut bytes = to_mem as usize;
        bytes *= 65536;
        let memvec = vec![0; bytes];
        let mut globs: Vec<StackTypes> = Vec::new(); 
        for global in &module.glob 
        {
            let mut gval = None;
            for c in &global.code{
                match c
                {
                    Code::I32Const(cons) => gval = Some(StackTypes::I32(*cons)),
                    Code::I64Const(cons) => gval = Some(StackTypes::I64(*cons)),
                    Code::F32Const(cons) => gval = Some(StackTypes::F32(*cons)),
                    Code::F64Const(cons) => gval = Some(StackTypes::F64(*cons)),
                    Code::End => break,
                    _ => panic!("Invalid Global"),
                }
            }
             globs.push(gval.expect("Error no Global Val new run"));
        }

        Runtime { module, mem: memvec, call_stack: Vec::new(), value_stack: Vec::new(), flow_stack: Vec::new(), globs,}
    }
    pub fn run_prog(&mut self) -> Option<StackTypes>
    {
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
        'run:
        loop {
            let call = match self.call_stack.last_mut()
            {
                Some(caller) => caller,
                None => {
                    //log::info!("End of wasm");
                    return None
                }
            };
            if call.loc >= call.code.len()
            {
                let turn = self.value_stack.pop();
                self.call_stack.pop();
                if self.call_stack.is_empty()
                {
                    return turn;
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
                //Code::Block(typ) => self.flow_stack.push({FlowCode { flow_type: FlowType::Block, break_tar: , size: self.value_stack.len(), ret_typ: typ}}),
                Code::Loop(typ) => self.flow_stack.push(FlowCode{ flow_type: FlowType::Loop, break_tar: call.loc - 1, size: self.value_stack.len(), ret_typ: typ,}),    
//                Code::If(typ) => //log::info!("If: {}", typ),
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
                        return turn;
                    }
                    if let Some(val) = turn
                    {
                        //log::info!("Return: {}", val);
                        self.value_stack.push(val);
                    }

                },
                Code::Call(ind) => 
                {
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
                    let func = &self.module.fcce[ind as usize];
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
                    self.call_stack.push(StackCalls{ fnid: ind as usize, code: fcode.to_vec(), loc: 0, vars});
                    continue 'run;
                },
                Code::CallIndirect(ind) => 
                {

                },
                //Args
                Code::Drop =>
                {
                    let _waste = self.value_stack.pop();
                },
                Code::Select =>
                {
                    let sel = match self.value_stack.pop(){
                        Some(StackTypes::I32(val)) => val,
                        _ => panic!("Stack Error Select"),
                    };
                    let val2 = self.value_stack.pop().expect("Stack Sel Fail");
                    let val1 = self.value_stack.pop().expect("Stack Sel Fail");

                    if sel != 0 {self.value_stack.push(val1);}
                    else{self.value_stack.push(val2);}
                },
                //Vars
                Code::LocalGet(loc) => {
                    let val = call.vars.get(loc as usize).unwrap().clone();
                    self.value_stack.push(val);
                    //log::info!("Local Get: Index: {}, Value: {}", loc, val);
                },
                Code::LocalSet(loc) => {
                    let to_stack = self.value_stack.pop().unwrap();
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
                    call.vars[ind] = to_loc;
                },
                Code::GlobalGet(loc) =>
                {
                    let to_stack = self.globs.get(loc as usize).cloned().expect("Couldnt get val globget");
                    self.value_stack.push(to_stack);
                    //log::info!("Global Get: Index: {}, Value: {}", loc, to_stack);
                },
                Code::GlobalSet(loc) =>
                {
                    let to_glob = self.value_stack.pop().expect("Stack empty globset");
                    self.globs[loc as usize] = to_glob;
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
                    let bytes = &self.mem[of..of + 4];
                    let to_stack = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                    let val = StackTypes::I32(to_stack);
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
                    self.mem[uloc..uloc + 4].copy_from_slice(&var.to_le_bytes()); 
                },
                Code::MemorySize => 
                {
                    let memlen = self.mem.len();
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
                    let newmem = (curmem + memchange) * 65536;
                    self.mem.resize(newmem as usize, 0);

                    self.value_stack.push(StackTypes::I32(curmem));

                },
                //Cons
                Code::I32Const(cons) => {
                    self.value_stack.push(StackTypes::I32(cons));
                    //log::info!("I32 Constant: {}", cons);
                },
                Code::I64Const(cons) => {
                    self.value_stack.push(StackTypes::I64(cons));
                    //log::info!("I64 Constant: {}", cons);
                },
                Code::F32Const(cons) => {
                    self.value_stack.push(StackTypes::F32(cons));
                    //log::info!("F32 Constant: {}", cons);
                },
                Code::F64Const(cons) => {
                    self.value_stack.push(StackTypes::F64(cons));
                    //log::info!("F64 Constant {}", cons);
                },
                //Comps
                //I32
                Code::I32Eqz => {
                    let i_val = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u32,
                        _ => panic!("Invalid type stack error"),
                    };
                    match i_val
                    {
                        0 => self.value_stack.push(StackTypes::I32(1)),
                        _ => self.value_stack.push(StackTypes::I32(0)),
                    }

                },
                Code::I32Eq => {
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
                    if val1 == val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I32Ne => {
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
                    if val1 >= val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                //I64
                Code::I64Eqz => {
                    let val = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v)) => v as u64,
                        _ => panic!("Invalid type stack error"),
                    };
                    if val == 0 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I64Eq => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v2)) => v2 as u64,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v1)) => v1 as u64,
                        _ => panic!("Invalid type stack error"),
                    };
                    if val1 == val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I64Ne => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v2)) => v2 as u64,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v1)) => v1 as u64,
                        _ => panic!("Invalid type stack error"),
                    };
                    if val1 != val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I64LtS => {
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
                    if val1 < val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I64LtU => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v2)) => v2 as u64,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v1)) => v1 as u64,
                        _ => panic!("Invalid type stack error"),
                    };
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
                    if val1 > val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I64GtU => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v2)) => v2 as u64,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v1)) => v1 as u64,
                        _ => panic!("Invalid type stack error"),
                    };
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
                    if val1 <= val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I64LeU => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v2)) => v2 as u64,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v1)) => v1 as u64,
                        _ => panic!("Invalid type stack error"),
                    };
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
                    if val1 >= val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                Code::I64GeU => {
                    let val2 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v2)) => v2 as u64,
                        _ => panic!("Invalid type stack error"),
                    };
                    let val1 = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(v1)) => v1 as u64,
                        _ => panic!("Invalid type stack error"),
                    };
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
                    if val1 <= val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
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
                    if val1 >= val2 {self.value_stack.push(StackTypes::I32(1));}
                    else {self.value_stack.push(StackTypes::I32(0));}
                },
                //Calcs
                //I32
//                Code::I32Clz => (),
//               Code::I32Ctz => (),
//                Code::I32Popcnt => (),
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
                    self.value_stack.push(StackTypes::I32(val1*val2));
                    //log::info!("I32 Multiplication: {} * {}", y, x);
                },
//                Code::I32DivS => (),
//                Code::I32DivU => (),
//                Code::I32RemS => (),
//                Code::I32RemU => (),
//                Code::I32And => (),
//                Code::I32Or => (),
//                Code::I32Xor => (),
//                Code::I32Shl => (),
//                Code::I32ShrS => (),
//                Code::I32ShrU => (),
//                Code::I32Rotl => (),
//                Code::I32Rotr => (),
                //I64
//                Code::I64Clz => (),
//                Code::I64Ctz => (),
//                Code::I64Popcnt => (),
//                Code::I64Add => (),
//                Code::I64Sub => (),
//                Code::I64Mul => (),
//                Code::I64DivS => (),
//                Code::I64DivU => (),
//                Code::I64RemS => (),
//                Code::I64RemU => (),
//                Code::I64And => (),
//                Code::I64Or => (),
//                Code::I64Xor => (),
//                Code::I64Shl => (),
//                Code::I64ShrS => (),
//                Code::I64ShrU => (),
//                Code::I64Rotl => (),
//                Code::I64Rotr => (),
                //FL
                //F32
//                Code::F32Abs => (),
//                Code::F32Neg => (),
//                Code::F32Ceil => (),
//                Code::F32Floor => (),
//                Code::F32Trunc => (),
//                Code::F32Nearest => (),
//                Code::F32Sqrt => (),
//                Code::F32Add => (),
//                Code::F32Sub => (),
//                Code::F32Mul => (),
//                Code::F32Div => (),
//                Code::F32Min => (),
//                Code::F32Max => (),
//                Code::F32Copysign => (),
                //F64
//                Code::F64Abs => (),
//                Code::F64Neg => (),
//                Code::F64Ceil => (),
//                Code::F64Floor => (),
//                Code::F64Trunc => (),
//                Code::F64Nearest => (),
//                Code::F64Sqrt => (),
//                Code::F64Add => (),
//                Code::F64Sub => (),
//                Code::F64Mul => (),
//                Code::F64Div => (),
//                Code::F64Min => (),
//                Code::F64Max => (),
//                Code::F64Copysign => (),
                //tools
                Code::I32WrapI64 => 
                {
                    let wrapped = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(val)) => val as i32,
                        _ => panic!("Invalid Stack Type I32WrapI64"),
                    };
                    self.value_stack.push(StackTypes::I32(wrapped));
                },
                Code::I32TruncF32S => 
                {
                    let trunced = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(val)) => val as i32,
                        _ => panic!("Invalid Stack Type I32WrapF32S"),
                    };
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
                    self.value_stack.push(StackTypes::I32(sender));
                },
                Code::I32TruncF64S => 
                {
                    let trunced = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(val)) => val as i32,
                        _ => panic!("Stack type is not a F64 I32TruncF64S"),
                    };
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
                    self.value_stack.push(StackTypes::I32(trunced as i32));
                },
                Code::I64ExtendI32S => 
                {
                    let extend = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as i64,
                        _ => panic!("Stack type is not I32 I64extendI32S"),
                    };
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
                    self.value_stack.push(StackTypes::I64(extend as i64));
                },
                Code::I64TruncF32S => 
                {
                    let trunced = match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(val)) => val as i64,
                        _ => panic!("Stack type is not F32 I64TruncF32S"),
                    };
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
                    self.value_stack.push(StackTypes::I64(trunced as i64));
                },
                Code::I64TruncF64S => 
                {
                    let trunced = match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(val)) => val as i64,
                        _ => panic!("Stack type is not F64 I64TruncF64S"),
                    };
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
                    self.value_stack.push(StackTypes::I64(trunced as i64));
                },
                Code::F32ConvertI32S => 
                {
                    let converted = match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as f32,
                        _ => panic!("Stack type is not I32 F32ConvertI32S"),
                    };
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
                    self.value_stack.push(StackTypes::F32(converted));
                },
                Code::F32ConvertI64S => 
                {
                    let converted = match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(val)) => val as f32,
                        _ => panic!("Stack type not I64 F32ConvertI64S"),
                    };
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
                    self.value_stack.push(StackTypes::F32(converted));
                },
                Code::F32DemoteF64 => 
                {
                    match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(val)) => self.value_stack.push(StackTypes::F32(val as f32)),
                        _ => panic!("Stack type not F64 F32DemoteF64"),
                    }
                },
                Code::F64ConvertI32S => 
                {
                    match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => self.value_stack.push(StackTypes::F64(val as f64)),
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
                            self.value_stack.push(StackTypes::F64(val as f64));
                        }
                        _ => panic!("Stack type not I32 F64ConvertI32U"),
                    }
                },
                Code::F64ConvertI64S => 
                {
                    match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(val)) => self.value_stack.push(StackTypes::F64(val as f64)),
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
                            self.value_stack.push(StackTypes::F64(val as f64));
                        },
                        _ => panic!("Stack type not I64 F64ConvertI64U"),
                    }
                },
                Code::F64PromoteF32 => 
                {
                    match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(val)) => self.value_stack.push(StackTypes::F64(val as f64)),
                        _ => panic!("Stack type not I32 F64PromoteF32"),
                    }
                },
                Code::I64ReinterpretF64 => 
                {
                    match self.value_stack.pop()
                    {
                        Some(StackTypes::F64(val)) => self.value_stack.push(StackTypes::I64(f64::to_bits(val) as i64)),
                        _ => panic!("Stack type not F64 I64ReinterpretF64"),
                    }
                },
                Code::I32ReinterpretF32 => 
                {
                    match self.value_stack.pop()
                    {
                        Some(StackTypes::F32(val)) => self.value_stack.push(StackTypes::I32(val.to_bits() as i32)),
                        _ => panic!("Stack type not F32 I32 ReinterpretF32"),
                    }
                },
                Code::F64ReinterpretI64 => 
                {
                    match self.value_stack.pop()
                    {
                        Some(StackTypes::I64(val)) => self.value_stack.push(StackTypes::F64(f64::from_bits(val as u64))),
                        _ => panic!("Stack type not I64 F64ReinterpretI64"),
                    }
                },
                Code::F32ReinterpretI32 => 
                {
                    match self.value_stack.pop()
                    {
                        Some(StackTypes::I32(val)) => self.value_stack.push(StackTypes::F32(f32::from_bits(val as u32))),
                        _ => panic!("Stack type not I32 F32ReinterpretI32"),
                    }
                },
                _ => panic!("Unsupported Type"),
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_types_i32() {
        let val = StackTypes::I32(42);
        if let StackTypes::I32(v) = val {
            assert_eq!(v, 42);
        } else {
            assert!(false, "Expected I32");
        }
    }
}

#[test]
    fn test_stack_types_i64() {
        let val = StackTypes::I64(1000000);
        if let StackTypes::I64(v) = val {
            assert_eq!(v, 1000000);
        } else {
            assert!(false, "Expected I64");
        }
    }

    #[test]
    fn test_stack_types_f32() {
        let val = StackTypes::F32(3.14);
        if let StackTypes::F32(v) = val {
            assert!((v - 3.14).abs() < 0.001);
        } else {
            assert!(false, "Expected F32");
        }
    }

    #[test]
    fn test_stack_types_f64() {
        let val = StackTypes::F64(2.71828);
        if let StackTypes::F64(v) = val {
            assert!((v - 2.71828).abs() < 0.0001);
        } else {
            assert!(false, "Expected F64");
        }
    }

    #[test]
    fn test_stack_types_clone() {
        let original = StackTypes::I32(123);
        let cloned = original.clone();
        if let (StackTypes::I32(v1), StackTypes::I32(v2)) = (original, cloned) {
            assert_eq!(v1, v2);
        }
    }

    #[test]
    fn test_stack_calls_basic() {
        let call = StackCalls {
            fnid: 0,
            code: vec![],
            loc: 0,
            vars: vec![],
        };
        assert_eq!(call.fnid, 0);
        assert_eq!(call.loc, 0);
        assert_eq!(call.code.len(), 0);
        assert_eq!(call.vars.len(), 0);
    }

    #[test]
    fn test_stack_calls_with_vars() {
        let call = StackCalls {
            fnid: 5,
            code: vec![],
            loc: 10,
            vars: vec![StackTypes::I32(42), StackTypes::I64(100)],
        };
        assert_eq!(call.fnid, 5);
        assert_eq!(call.loc, 10);
        assert_eq!(call.vars.len(), 2);
    }

    #[test]
    fn test_flow_code_block() {
        let flow = FlowCode {
            flow_type: FlowType::Block,
            break_tar: 15,
            size: 3,
            ret_typ: None,
        };
        assert_eq!(flow.break_tar, 15);
        assert_eq!(flow.size, 3);
    }

    #[test]
    fn test_flow_code_if() {
        let flow = FlowCode {
            flow_type: FlowType::If,
            break_tar: 5,
            size: 1,
            ret_typ: Some(TypeBytes::I32),
        };
        assert_eq!(flow.break_tar, 5);
        assert_eq!(flow.size, 1);
    }

    #[test]
    fn test_flow_code_loop() {
        let flow = FlowCode {
            flow_type: FlowType::Loop,
            break_tar: 0,
            size: 10,
            ret_typ: Some(TypeBytes::I64),
        };
        assert_eq!(flow.break_tar, 0);
        assert_eq!(flow.size, 10);
    }