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
    pub calls: Vec<StackCalls>,
    pub stack: Vec<StackTypes>,
    pub globs: Vec<StackTypes>,

}
impl Runtime
{
    pub fn new(module: Module) -> Self
    {
        let to_mem = module.memy.get(0).map(|(min, _max)| *min).unwrap_or(1);
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

        Runtime { module, mem: memvec, calls: Vec::new(), stack: Vec::new(), globs,}
    }
    pub fn run_prog(&mut self) -> Option<StackTypes>
    {
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
            self.calls.push(StackCalls { fnid: strtind, code: func.code.clone(), loc: 0, vars,});
        }
        'run:
        loop {
            let call = match self.calls.last_mut()
            {
                Some(caller) => caller,
                None => return None,
            };
            if call.loc >= call.code.len()
            {
                let turn = self.stack.pop();
                self.calls.pop();
                if self.calls.is_empty()
                {
                    return turn;
                }
                if let Some(turner) = turn {self.stack.push(turner)}
                continue 'run;
            }
            let code = call.code[call.loc].clone();
            call.loc += 1;
            match code
            {
                Code::Unreachable => (),
                Code::Nop => (),
                Code::Block(typ) => (),
                Code::Loop(typ) => (),
                Code::If(typ) => (),
                Code::Else => (),
                Code::Br(us) => (),
                Code::BrIf(us) => (),
                //Code::BrTable => (),
                Code::Return | Code::End =>
                {
                    let turn = self.stack.pop();
                    self.calls.pop();
                    if self.calls.is_empty(){return turn;}
                    if let Some(val) = turn
                    {
                        self.stack.push(val);
                    }

                }
                Code::Call(ind) => 
                {
                    let typind = self.module.fnid[ind as usize] as usize;
                    let typ = &self.module.typs[typind];
                    let mut cvec = Vec::new();
                    let mut itt = 0;
                    while itt < typ.args.len()
                    {
                        cvec.push(self.stack.pop().unwrap());
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
                    self.calls.push(StackCalls{ fnid: ind as usize, code: fcode.to_vec(), loc: 0, vars});
                    continue 'run;
                }
                Code::I32Const(cons) => self.stack.push(StackTypes::I32(cons)),
                Code::I64Const(cons) => self.stack.push(StackTypes::I64(cons)),
                Code::F32Const(cons) => self.stack.push(StackTypes::F32(cons)),
                Code::F64Const(cons) => self.stack.push(StackTypes::F64(cons)),
                Code::LocalGet(loc) => {
                    let val = call.vars.get(loc as usize).unwrap().clone();
                    self.stack.push(val)
                },
                Code::LocalSet(loc) => call.vars[loc as usize] = self.stack.pop().unwrap(),
                Code::GlobalGet(loc) =>
                {
                    let to_stack = self.globs.get(loc as usize).cloned().expect("Couldnt get val globget");
                    self.stack.push(to_stack);
                }
                Code::GlobalSet(loc) =>
                {
                    let to_glob = self.stack.pop().expect("Stack empty globset");
                    self.globs[loc as usize] = to_glob;
                }
                Code::I32Add => {
                    let x = match self.stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val,
                        _ => panic!("Add error"),
                    };
                    let y = match self.stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val,
                        _ => panic!("Add error"),
                    };
                    self.stack.push(StackTypes::I32(y+x));
                }
                Code::I32Sub =>
                {
                    let x = match self.stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val,
                        _ => panic!("Sub error"),
                    };
                    let y = match self.stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val,
                        _ => panic!("Sub error"),
                    };
                    self.stack.push(StackTypes::I32(y-x));
                }
                Code::I32Mul =>
                {
                    let x = match self.stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val,
                        _ => panic!("Mul error"),
                    };
                    let y = match self.stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val,
                        _ => panic!("Mul error"),
                    };    
                    self.stack.push(StackTypes::I32(y*x));
                }
                Code::I32Load(off) =>
                {
                    let memloc = match self.stack.pop() {
                        Some(StackTypes::I32(loc)) => loc,
                        _ => panic!("Mem error"),
                    };
                    let offloc = off + memloc as u32;  
                    let of = offloc as usize;              
                    let bytes = &self.mem[of..of + 4];
                    let to_stack = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                    self.stack.push(StackTypes::I32(to_stack));
                }
                Code::I32Store(off) =>
                {
                    let memloc = match self.stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u32,
                        _ => panic!("Store Stack Err"),
                    };
                    let var = match self.stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val,
                        _ => panic!("Store Stack err"),
                    };
                    let uloc = (off + memloc) as usize;
                    let bytes = var.to_le_bytes();
                    self.mem[uloc..uloc + 4].copy_from_slice(&bytes);
                }
                Code::I64Load(off) =>
                {
                    let memloc = match self.stack.pop() {
                        Some(StackTypes::I32(loc)) => loc,
                        _ => panic!("Mem error"),
                    };
                    let offloc = off + memloc as u32;  
                    let of = offloc as usize;              
                    let bytes = &self.mem[of..of + 8];
                    let to_stack = i64::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]]);
                    self.stack.push(StackTypes::I64(to_stack));
                }
                Code::I64Store(off) =>
                {
                    let memloc = match self.stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u32,
                        _ => panic!("Store Stack Err"),
                    };
                    let var = match self.stack.pop()
                    {
                        Some(StackTypes::I64(val)) => val,
                        _ => panic!("Store Stack err"),
                    };
                    let uloc = (off + memloc) as usize;
                    let bytes = var.to_le_bytes();
                    self.mem[uloc..uloc + 8].copy_from_slice(&bytes);
                }
                Code::F32Load(off) => 
                {
                    let memloc = match self.stack.pop() {
                    Some(StackTypes::I32(loc)) => loc as u32,
                    _ => panic!("Mem error"),
                    };
                    let offloc = off + memloc;  
                    let of = offloc as usize;              
                    let bytes = &self.mem[of..of + 4];
                    let to_stack = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                    self.stack.push(StackTypes::F32(to_stack));
                }
                Code::F32Store(off) =>
                {
                    let memloc = match self.stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u32,
                        _ => panic!("Store Stack Err"),
                    };
                    let var = match self.stack.pop()
                    {
                        Some(StackTypes::F32(val)) => val,
                        _ => panic!("Store Stack err"),
                    };
                    let uloc = (off + memloc) as usize;
                    let bytes = var.to_le_bytes();
                    self.mem[uloc..uloc + 4].copy_from_slice(&bytes);                    
                }
                Code::F64Load(off) =>
                {
                    let memloc = match self.stack.pop() {
                        Some(StackTypes::I32(loc)) => loc as u32,
                        _ => panic!("Mem error"),
                    };
                    let offloc = off + memloc;  
                    let of = offloc as usize;              
                    let bytes = &self.mem[of..of + 8];
                    let to_stack = f64::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]]);
                    self.stack.push(StackTypes::F64(to_stack));
                }
                Code::F64Store(off) =>
                {
                    let memloc = match self.stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val as u32,
                        _ => panic!("Store Stack Err"),
                    };
                    let var = match self.stack.pop()
                    {
                        Some(StackTypes::F64(val)) => val,
                        _ => panic!("Store Stack err"),
                    };
                    let uloc = (off + memloc) as usize;
                    let bytes = var.to_le_bytes();
                    self.mem[uloc..uloc + 8].copy_from_slice(&bytes);
                }
                
                Code::Drop =>
                {
                    let _waste = self.stack.pop();
                }
                Code::Select =>
                {
                    let sel = match self.stack.pop(){
                        Some(StackTypes::I32(val)) => val,
                        _ => panic!("Stack Error Select"),
                    };
                    let y = self.stack.pop().expect("Stack Sel Fail");
                    let x = self.stack.pop().expect("Stack Sel Fail");

                    if sel != 0 {self.stack.push(x);}
                    else{self.stack.push(y);}
                }
                Code::LocalTee(loc) =>
                {
                    let to_loc = self.stack.last().cloned().expect("Local Tee stk error");
                    let ind = loc as usize;
                    if ind >= call.vars.len()
                    {
                        panic!("LocalT: Index out of calls");
                    }
                    call.vars[ind] = to_loc;
                }
                Code::MemorySize => 
                {
                    let memlen = self.mem.len();
                    self.stack.push(StackTypes::I32((memlen/65536) as i32));
                }
                Code::MemoryGrow => 
                {
                    let memchange = match self.stack.pop()
                    {
                        Some(StackTypes::I32(change)) => change,
                        _ => panic!("Invalid type memchange"),
                    };
                    assert!(memchange >= 0);
                    let curmem = (self.mem.len()/65536) as i32;
                    let newmem = (curmem + memchange) * 65536;
                    self.mem.resize(newmem as usize, 0);

                    self.stack.push(StackTypes::I32(curmem));

                }

                _ => panic!("Unsupported Type"),
            }
        }
    }

}