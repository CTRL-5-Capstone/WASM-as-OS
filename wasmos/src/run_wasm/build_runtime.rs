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
           // for c in global.code{
                /*let styp = match{
                    Code::
                }*/
            //}
        }

        Runtime { module, mem: memvec, calls: Vec::new(), stack: Vec::new(), globs: Vec::new()}
    }
    pub fn run_prog(&mut self) -> Option<StackTypes>
    {
        if let Some(starter) = self.module.strt
        {

            let strtind = (starter - self.module.imports) as usize;
            let typin = self.module.fnid[strtind] as usize;
            let typ = &self.module.typs[typin];
            let func = &self.module.fcce[strtind];
            let vars = Vec::new();
            for (_loc, typ) in &func.vars
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
                self.calls.pop();
                return self.stack.pop();
                
            }
            let code = call.code[call.loc].clone();
            call.loc += 1;
            match code
            {
                Code::I32Const(cons) => self.stack.push(StackTypes::I32(cons)),
                Code::LocalGet(loc) => {
                    let val = call.vars.get(loc as usize).unwrap().clone();
                    self.stack.push(val)
                },
                Code::LocalSet(loc) => call.vars[loc as usize] = self.stack.pop().unwrap(),
                Code::I32Add => {
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
                    self.stack.push(StackTypes::I32(x+y));
                }
                Code::I32Sub =>
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
                    self.stack.push(StackTypes::I32(x-y));
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
                    self.stack.push(StackTypes::I32(x*y));
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
                    let var = match self.stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val,
                        _ => panic!("Store Stack err"),
                    };
                    let memloc = match self.stack.pop()
                    {
                        Some(StackTypes::I32(val)) => val,
                        _ => panic!("Store Stack Err"),
                    };
                    let uloc = memloc as usize;
                    let bytes = var.to_le_bytes();
                    self.mem[uloc..uloc + 4].copy_from_slice(&bytes);
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
                    let mut fvars= Vec::new();
                    fvars.extend_from_slice(&cvec);
                    let func = &self.module.fcce[ind as usize];
                    let fcode = &func.code;

                    self.calls.push(StackCalls{ fnid: ind as usize, code: fcode.to_vec(), loc: 0, vars: fvars});
                    continue 'run;
                }
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
                _ => panic!("Unsupported Type"),
            }
        }
    }

}