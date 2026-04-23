use core::panic;
use super::wasm_module::*;
use super::syscall_policy::{SyscallPolicy, SyscallViolation, PolicyAction};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use serde::{Serialize, Deserialize};
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum StackTypes
{
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}
#[derive(Clone, Serialize, Deserialize)]
pub struct StackCalls
{
    pub fnid: usize,
    pub code: Vec<Code>,
    pub loc: usize,
    pub vars: Vec<StackTypes>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct GlobsGlobal
{
    typ: StackTypes, 
    ismut: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Runtime
{
    pub paused: bool,
    pub incount: usize,
    pub ended: bool,
    pub priority: usize,
    pub limflag: bool,
    pub limit: usize,
    pub flog: bool,
    pub clog: bool,
    pub module: Module,
    pub mem: Vec<u8>,
    pub memmin: u32,
    pub memmax: Option<u32>,
    pub call_stack: Vec<StackCalls>, 
    pub value_stack: Vec<StackTypes>,
    pub flow_stack: Vec<FlowCode>,
    pub globs: Vec<GlobsGlobal>,
    pub functab: Vec<Option<u32>>,
    // Execution metrics (merged from original)
    pub instruction_count: u64,
    pub syscall_count: u64,
    pub stdout_log: Vec<String>,
    // ── Syscall filtering ────────────────────────────────────────────────────
    /// Policy that governs which imports are allowed to execute.
    #[serde(skip)]
    pub policy: SyscallPolicy,
    /// All recorded policy violations for this execution.
    pub violations: Vec<SyscallViolation>,
}
#[derive(Clone, Serialize, Deserialize)]
pub enum FlowType
{
    If,
    Block,
    Loop,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct FlowCode
{
    flow_type: FlowType,
    break_tar: usize,
    size: usize,
    ret_typ: Option<TypeBytes>
    
}
impl Runtime
{
    /// Create a new Runtime with an optional syscall policy.
    /// Pass `None` (or `SyscallPolicy::permissive()`) to preserve the legacy allow-all behaviour.
    pub fn new_with_policy(module: Module, policy: SyscallPolicy) -> Self
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
        // Hard cap: WASM spec allows up to 65536 pages (4 GB) but we limit to 2048 pages
        // (128 MB) to match the config.limits.max_memory_mb default and prevent OOM from
        // malicious or runaway modules declaring absurdly large memory.
        const MAX_WASM_MEMORY_PAGES: usize = 2048; // 128 MB
        let capped_pages = (memmin as usize).min(MAX_WASM_MEMORY_PAGES);
        let bytes = capped_pages * 65536;
        let mut memvec = vec![0u8; bytes];
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
            let gval: StackTypes = match global.code
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
            assert!(off + elm.fvec.len() <= functab.len(), 
                "Element init out of bounds: off={} fvec_len={} functab_len={}", 
                off, elm.fvec.len(), functab.len());
            for byts in &elm.fvec
            {
                if off < functab.len() {
                    functab[off] = Some(*byts);
                }
                off +=1;
            }
        }
        Runtime{
            paused: false, incount: 0, ended: false, priority: 1,
            flog: false, clog: false, limflag: false, limit: 0,
            module, functab, mem: memvec, memmin, memmax,
            call_stack: Vec::new(), value_stack: Vec::new(), flow_stack: Vec::new(),
            globs,
            instruction_count: 0, syscall_count: 0, stdout_log: Vec::new(),
            policy,
            violations: Vec::new(),
        }
    }
    /// Backward-compatible constructor — defaults to the permissive (allow-all) policy.
    pub fn new(module: Module) -> Self {
        Self::new_with_policy(module, SyscallPolicy::permissive())
    } 
    pub fn pop_run(&mut self)
    {
        // Priority 1: explicit WASM `start` section
        let entry_func_idx: usize = if let Some(starter) = self.module.strt
        {
            (starter - self.module.imports) as usize
        }
        else
        {
            // Priority 2: exported function named "start", "_start", or "main"
            let export_entry = self.module.exps.iter().find(|e| {
                matches!(e.typ, super::wasm_module::ExpTyp::Func)
                    && (e.name == "start" || e.name == "_start" || e.name == "main")
            });

            if let Some(exp) = export_entry {
                // exp.loc is the absolute function index (imports + defined functions)
                (exp.loc.saturating_sub(self.module.imports)) as usize
            } else {
                // Priority 3: fall back to the first defined function
                0
            }
        };

        if self.module.fcce.is_empty() || entry_func_idx >= self.module.fcce.len() {
            // Module has no runnable functions — mark as already ended.
            self.ended = true;
            return;
        }

        let _abs_idx = self.module.imports as usize + entry_func_idx;
        let func = &self.module.fcce[entry_func_idx];
        let mut vars = Vec::new();

        // Push default-zero locals for each declared local variable group
        for (loc, typ) in &func.vars
        {
            let ty = match typ
            {
                Some(typ) => typ,
                None => continue, // skip invalid local declarations
            };
            let styp = match ty
            {
                super::wasm_module::TypeBytes::I32 => StackTypes::I32(0),
                super::wasm_module::TypeBytes::I64 => StackTypes::I64(0),
                super::wasm_module::TypeBytes::F32 => StackTypes::F32(0.0),
                super::wasm_module::TypeBytes::F64 => StackTypes::F64(0.0),
            };
            for _ in 0..*loc
            {
                vars.push(styp.clone());
            }
        }

        // Also push default-zero values for any function parameters
        // (the entry function is called with no arguments from the host)
        // Look up the type index via fnid, not using the absolute function index directly
        let type_idx_opt = if entry_func_idx < self.module.fnid.len() {
            Some(self.module.fnid[entry_func_idx] as usize)
        } else {
            None
        };
        if let Some(type_idx) = type_idx_opt {
            if type_idx < self.module.typs.len() {
                // Only prepend param slots if vars doesn't already cover them
                // (params come before locals in the frame)
                let mut param_vars: Vec<StackTypes> = Vec::new();
                for arg in &self.module.typs[type_idx].args {
                    let v = match arg {
                        Some(super::wasm_module::TypeBytes::I32) => StackTypes::I32(0),
                        Some(super::wasm_module::TypeBytes::I64) => StackTypes::I64(0),
                        Some(super::wasm_module::TypeBytes::F32) => StackTypes::F32(0.0),
                        Some(super::wasm_module::TypeBytes::F64) => StackTypes::F64(0.0),
                        None => StackTypes::I32(0),
                    };
                    param_vars.push(v);
                }
                param_vars.extend(vars);
                vars = param_vars;
            }
        }

        self.call_stack.push(StackCalls {
            fnid: entry_func_idx,
            code: func.code.clone(),
            loc: 0,
            vars,
        });
    }
    fn alogger(&mut self, logg: String)
    {
        if self.flog
        {
            let pstring = format!("{}{}{}", "./wasm_files/", self.module.name, ".txt");
            let path = Path::new(&pstring);
            if let Ok(mut wasfile) = OpenOptions::new().create(true).append(true).open(path)
            {
                writeln!(&mut wasfile, "{logg}").expect("Log could not be written");
            }
            else
            {
                println!("Runtime: {} File Write Error", self.module.name);
            }
        }
        if self.clog
        {

        }
    }
    /// Find the position of the matching End for a Block/If starting at `start_loc`.
    fn find_matching_end(code: &[Code], start_loc: usize) -> usize {
        let mut depth = 1u32;
        let mut pos = start_loc;
        while pos < code.len() {
            match &code[pos] {
                Code::Block(_) | Code::Loop(_) | Code::If(_) => depth += 1,
                Code::End => {
                    depth -= 1;
                    if depth == 0 {
                        return pos;
                    }
                }
                _ => {}
            }
            pos += 1;
        }
        code.len() - 1 // fallback
    }
    /// Find the Else and End positions for an If block starting at `start_loc`.
    fn find_else_and_end(code: &[Code], start_loc: usize) -> (Option<usize>, usize) {
        let mut depth = 1u32;
        let mut else_pos = None;
        let mut pos = start_loc;
        while pos < code.len() {
            match &code[pos] {
                Code::Block(_) | Code::Loop(_) | Code::If(_) => depth += 1,
                Code::Else => {
                    if depth == 1 {
                        else_pos = Some(pos);
                    }
                }
                Code::End => {
                    depth -= 1;
                    if depth == 0 {
                        return (else_pos, pos);
                    }
                }
                _ => {}
            }
            pos += 1;
        }
        (else_pos, code.len() - 1)
    }
    /// Execute a branch to the given depth.
    fn do_branch(&mut self, depth: u32) {
        // Pop `depth` flow frames (target is the (depth+1)th from top)
        for _ in 0..depth {
            self.flow_stack.pop();
        }
        if let Some(target_flow) = self.flow_stack.pop() {
            match target_flow.flow_type {
                FlowType::Loop => {
                    // For loops, re-push and jump to loop start
                    let break_tar = target_flow.break_tar;
                    self.flow_stack.push(target_flow);
                    let call = self.call_stack.last_mut().unwrap();
                    call.loc = break_tar;
                }
                FlowType::Block | FlowType::If => {
                    // For blocks/ifs, jump past End
                    let break_tar = target_flow.break_tar;
                    // Restore value stack
                    if target_flow.ret_typ.is_some() {
                        let ret_val = self.value_stack.pop();
                        while self.value_stack.len() > target_flow.size {
                            self.value_stack.pop();
                        }
                        if let Some(val) = ret_val {
                            self.value_stack.push(val);
                        }
                    } else {
                        while self.value_stack.len() > target_flow.size {
                            self.value_stack.pop();
                        }
                    }
                    let call = self.call_stack.last_mut().unwrap();
                    call.loc = break_tar + 1; // skip past End
                }
            }
        } else {
            // Branch out of function
            let ret_val = self.value_stack.pop();
            self.call_stack.pop();
            if self.call_stack.is_empty() {
                if let Some(val) = ret_val {
                    self.value_stack.push(val);
                }
                self.ended = true;
            } else if let Some(val) = ret_val {
                self.value_stack.push(val);
            }
        }
    }

    // ── Type-coercing stack pop helpers ──────────────────────────────────────
    // These extract a value from the stack, coercing from any numeric type
    // rather than panicking on a type mismatch.  This lets the engine run
    // real-world WASM that may leave an unexpected type on the stack (e.g.
    // after a stubbed CallIndirect return).
    #[inline]
    fn pop_i32(&mut self) -> i32 {
        match self.value_stack.pop() {
            Some(StackTypes::I32(v)) => v,
            Some(StackTypes::I64(v)) => v as i32,
            Some(StackTypes::F32(v)) => v as i32,
            Some(StackTypes::F64(v)) => v as i32,
            None => 0,
        }
    }
    #[inline]
    fn pop_u32(&mut self) -> u32 {
        self.pop_i32() as u32
    }
    #[inline]
    fn pop_i64(&mut self) -> i64 {
        match self.value_stack.pop() {
            Some(StackTypes::I64(v)) => v,
            Some(StackTypes::I32(v)) => v as i64,
            Some(StackTypes::F32(v)) => v as i64,
            Some(StackTypes::F64(v)) => v as i64,
            None => 0,
        }
    }
    #[allow(dead_code)]
    #[inline]
    fn pop_u64(&mut self) -> u64 {
        self.pop_i64() as u64
    }
    #[inline]
    fn pop_f32(&mut self) -> f32 {
        match self.value_stack.pop() {
            Some(StackTypes::F32(v)) => v,
            Some(StackTypes::F64(v)) => v as f32,
            Some(StackTypes::I32(v)) => v as f32,
            Some(StackTypes::I64(v)) => v as f32,
            None => 0.0,
        }
    }
    #[inline]
    fn pop_f64(&mut self) -> f64 {
        match self.value_stack.pop() {
            Some(StackTypes::F64(v)) => v,
            Some(StackTypes::F32(v)) => v as f64,
            Some(StackTypes::I32(v)) => v as f64,
            Some(StackTypes::I64(v)) => v as f64,
            None => 0.0,
        }
    }

    pub fn run_prog(&mut self)
    {
        let mut lstring: String = "".to_string();
        if self.call_stack.is_empty()
        {
            self.ended = true;
            return;
        }
        let call = self.call_stack.last_mut().unwrap();
        if call.loc >= call.code.len() {
            // Implicit return — pop the call frame
            self.call_stack.pop();
            return;
        }
        let code = call.code[call.loc].clone();
        call.loc += 1;
        self.instruction_count += 1;
        match code
        {
            //flow
            Code::Unreachable => {
                // WASM trap instruction — terminate execution gracefully.
                // In real WASM runtimes this is a trap (e.g. Rust's compiled panic!).
                // The engine handles it by ending execution rather than panicking.
                self.stdout_log.push("[TRAP] WASM unreachable instruction executed".to_string());
                self.ended = true;
                return;
            }
            Code::Nop => (), //instruction is a placeholder in wasm
            Code::Block(typ) => {
                // Find the matching End for this Block to set break_tar
                let break_tar = Self::find_matching_end(&call.code, call.loc);
                self.flow_stack.push(FlowCode{flow_type: FlowType::Block, break_tar, size: self.value_stack.len(), ret_typ: typ});
            },
            Code::Loop(typ) => self.flow_stack.push(FlowCode{ flow_type: FlowType::Loop, break_tar: call.loc, size: self.value_stack.len(), ret_typ: typ,}),    
            Code::If(typ) => {
                // Drop `call` before calling pop_i32() — both need &mut self.
                let call_loc_snapshot = call.loc;
                let _ = call; // release mutable borrow before pop_i32() needs &mut self
                let condition = self.pop_i32();
                // Reborrow (immutably) just to scan for Else/End positions.
                let (else_pos, end_pos) = {
                    let call = self.call_stack.last().unwrap();
                    Self::find_else_and_end(&call.code, call_loc_snapshot)
                };
                if condition != 0 {
                    // True branch: push If flow frame, break target is End
                    self.flow_stack.push(FlowCode{flow_type: FlowType::If, break_tar: end_pos, size: self.value_stack.len(), ret_typ: typ});
                } else {
                    // False branch: jump to Else+1 or End
                    if let Some(ep) = else_pos {
                        // Push If flow frame for the else branch
                        self.flow_stack.push(FlowCode{flow_type: FlowType::If, break_tar: end_pos, size: self.value_stack.len(), ret_typ: typ});
                        let call = self.call_stack.last_mut().unwrap();
                        call.loc = ep + 1; // skip past the Else opcode
                    } else {
                        // No else branch — skip to End
                        let call = self.call_stack.last_mut().unwrap();
                        call.loc = end_pos + 1; // skip past End
                    }
                }
            },
            Code::Else => {
                // Hit Else during true branch execution — jump to End
                if let Some(flow) = self.flow_stack.last() {
                    let end_pos = flow.break_tar;
                    self.flow_stack.pop();
                    let call = self.call_stack.last_mut().unwrap();
                    call.loc = end_pos + 1; // skip past End
                }
            },
            Code::Br(depth) => 
            {
                self.do_branch(depth);
            }
            Code::BrIf(depth) => 
            {
                let boo = self.pop_i32();
                if boo != 0
                {
                    self.do_branch(depth);
                }
                // if boo == 0, fall through (do nothing)
            },
            Code::BrTable { def, ref locs } => {
                let index = match self.value_stack.pop() {
                    Some(StackTypes::I32(v)) => v as u32,
                    _ => panic!("Expected I32 for br_table index"),
                };
                let depth = if (index as usize) < locs.len() {
                    locs[index as usize]
                } else {
                    def
                };
                self.do_branch(depth);
            },
            Code::Return => {
                // Only pop a return value if this function's type signature has a return
                let has_return = if let Some(call) = self.call_stack.last() {
                    if call.fnid < self.module.fnid.len() {
                        let type_idx = self.module.fnid[call.fnid] as usize;
                        type_idx < self.module.typs.len() && !self.module.typs[type_idx].turns.is_empty()
                    } else {
                        false
                    }
                } else {
                    false
                };
                let ret_val = if has_return { self.value_stack.pop() } else { None };
                // Pop flow frames that belong to this function call
                // (all flow frames pushed since the current call frame was entered)
                // We track this by popping all remaining flow frames — in a well-formed
                // WASM module, a Return should have unwound any blocks/loops.
                // To be safe, pop up to the value_stack size recorded at call entry.
                while !self.flow_stack.is_empty() {
                    self.flow_stack.pop();
                }
                self.call_stack.pop();
                if self.call_stack.is_empty() {
                    if let Some(val) = ret_val {
                        self.value_stack.push(val);
                    }
                    self.ended = true;
                    return;
                }
                if let Some(val) = ret_val {
                    self.value_stack.push(val);
                }
            },
            Code::End =>
            {
                if let Some(flow) = self.flow_stack.pop() {
                    // End of a Block/Loop/If — restore value stack to expected size
                    // If the block has a return type, keep the top value
                    if flow.ret_typ.is_some() {
                        let ret_val = self.value_stack.pop();
                        while self.value_stack.len() > flow.size {
                            self.value_stack.pop();
                        }
                        if let Some(val) = ret_val {
                            self.value_stack.push(val);
                        }
                    } else {
                        while self.value_stack.len() > flow.size {
                            self.value_stack.pop();
                        }
                    }
                } else {
                    // End of function — only pop return value if function has a return type
                    let has_return = if let Some(call) = self.call_stack.last() {
                        if call.fnid < self.module.fnid.len() {
                            let type_idx = self.module.fnid[call.fnid] as usize;
                            type_idx < self.module.typs.len() && !self.module.typs[type_idx].turns.is_empty()
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    let ret_val = if has_return { self.value_stack.pop() } else { None };
                    self.call_stack.pop();
                    if self.call_stack.is_empty() {
                        if let Some(val) = ret_val {
                            self.value_stack.push(val);
                        }
                        self.ended = true;
                        return;
                    }
                    if let Some(val) = ret_val {
                        self.value_stack.push(val);
                    }
                }
            },
            Code::Call(ind) => 
            {
                lstring = format!("{}. Call {}", self.incount, ind);
                // Check if this is an import call (ABI syscall)
                if (ind as u32) < self.module.imports {
                    let imp = &self.module.imps[ind as usize];
                    let imp_name = imp.impname.clone();
                    let mod_name = imp.modname.clone();

                    // ── Policy enforcement ────────────────────────────────────────────────
                    if let PolicyAction::Deny = self.policy.check(&imp_name) {
                        // Record the violation
                        if self.violations.len() < self.policy.max_violations {
                            self.violations.push(SyscallViolation::new(
                                &imp_name,
                                &mod_name,
                                self.instruction_count,
                                format!(
                                    "Blocked by '{}' policy (default={:?})",
                                    self.policy.label,
                                    self.policy.default_action,
                                ),
                            ));
                        }
                        // Log to stdout so the UI can surface it
                        let blocked_msg = format!(
                            "[SYSCALL BLOCKED] '{}::{}' denied by {} policy at instruction {}",
                            mod_name, imp_name, self.policy.label, self.instruction_count,
                        );
                        println!("{}", blocked_msg);
                        self.stdout_log.push(blocked_msg);
                        // Halt execution immediately — this is a hard security boundary.
                        self.ended = true;
                        return;
                    }
                    // ── Dispatch allowed import ───────────────────────────────────────────
                    match imp_name.as_str() {
                        "host_log" => {
                            // ABI: host_log(ptr: i32, len: i32)
                            let len = match self.value_stack.pop() {
                                Some(StackTypes::I32(v)) => v as usize,
                                _ => panic!("Invalid type stack error"),
                            };
                            let ptr = match self.value_stack.pop() {
                                Some(StackTypes::I32(v)) => v as usize,
                                _ => panic!("Invalid type stack error"),
                            };
                            if ptr + len <= self.mem.len() {
                                let msg = String::from_utf8_lossy(&self.mem[ptr..ptr + len]).to_string();
                                println!("[WASM LOG] {}", msg);
                                self.stdout_log.push(msg);
                            } else {
                                self.stdout_log.push("[host_log: out of bounds]".to_string());
                            }
                            self.syscall_count += 1;
                        }
                        "read_sensor" => {
                            // ABI: read_sensor(sensor_id: i32) -> i32
                            let _sensor_id = self.pop_i32();
                            self.value_stack.push(StackTypes::I32(42));
                            self.syscall_count += 1;
                        }
                        "send_alert" => {
                            // ABI: send_alert(code: i32)
                            let code = self.pop_i32();
                            let msg = format!("[ALERT] code={}", code);
                            println!("{}", msg);
                            self.stdout_log.push(msg);
                            self.syscall_count += 1;
                        }
                        _ => {
                            // Unknown import that passed the policy check (i.e. policy is
                            // permissive or it was explicitly allowed).  Stub it out rather
                            // than panicking — pop args, push 0 if a return is expected.
                            let imp = &self.module.imps[ind as usize];
                            if let Some(type_idx) = imp.index {
                                if let Some(typ) = self.module.typs.get(type_idx as usize) {
                                    for _ in 0..typ.args.len() {
                                        self.value_stack.pop();
                                    }
                                    if !typ.turns.is_empty() {
                                        self.value_stack.push(StackTypes::I32(0));
                                    }
                                }
                            }
                            // Log allowed-but-unknown calls for observability.
                            if self.policy.log_allowed {
                                self.stdout_log.push(format!("[import stub] {}::{}", mod_name, imp_name));
                            }
                            self.syscall_count += 1;
                        }
                    }
                    return;
                }
                // Regular function call — fnid is indexed by LOCAL function index
                // (relative to the start of defined functions, not counting imports).
                let local_fn_idx = (ind as usize) - (self.module.imports as usize);
                if local_fn_idx >= self.module.fnid.len() {
                    // Malformed WASM: function index out of range — skip gracefully.
                    return;
                }
                let typind = self.module.fnid[local_fn_idx] as usize;
                let typ = &self.module.typs[typind];
                let mut cvec = Vec::new();
                let mut itt = 0;
                while itt < typ.args.len()
                {
                    let val = self.value_stack.pop().unwrap_or(StackTypes::I32(0));
                    cvec.push(val);
                    itt += 1;
                }
                cvec.reverse();
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
            },
            /*Code::CallIndirect(ind) => 
            {

            },*/
            Code::CallIndirect(type_idx) => {
                lstring = format!("{}. CallIndirect type={}", self.incount, type_idx);
                // Pop the table index from the value stack
                let table_index = match self.value_stack.pop() {
                    Some(StackTypes::I32(v)) => v as usize,
                    Some(other) => {
                        // Non-i32 index — treat as 0 (best-effort)
                        let _ = other;
                        0usize
                    },
                    None => {
                        // Empty stack — skip this call gracefully
                        self.value_stack.push(StackTypes::I32(0));
                        return;
                    }
                };
                // Look up function index from the function table — bounds-check first
                if table_index >= self.functab.len() {
                    // Table slot out of range — push stub return and continue
                    self.value_stack.push(StackTypes::I32(0));
                    return;
                }
                let func_idx = match self.functab[table_index] {
                    Some(idx) => idx,
                    None => {
                        // Uninitialized table slot — push stub return and continue
                        self.value_stack.push(StackTypes::I32(0));
                        return;
                    }
                };
                // Bounds-check func_idx against the function table
                if func_idx as usize >= self.module.fnid.len() {
                    self.value_stack.push(StackTypes::I32(0));
                    return;
                }
                // Validate type signature matches (best-effort — log mismatch but continue)
                // fnid is indexed by LOCAL function index (subtract imports from absolute index).
                let local_fn_idx_for_type = (func_idx as usize).saturating_sub(self.module.imports as usize);
                let actual_type_idx = if local_fn_idx_for_type < self.module.fnid.len() {
                    self.module.fnid[local_fn_idx_for_type] as usize
                } else {
                    0
                };
                // Set up the call like a regular Call
                let typ_args_len = if actual_type_idx < self.module.typs.len() {
                    self.module.typs[actual_type_idx].args.len()
                } else {
                    0
                };
                // Bounds-check func_idx against the code section (local functions only)
                let local_idx = (func_idx as u32).saturating_sub(self.module.imports) as usize;
                if local_idx >= self.module.fcce.len() {
                    // This is an imported function call via call_indirect — stub it
                    for _ in 0..typ_args_len {
                        let _ = self.value_stack.pop();
                    }
                    self.value_stack.push(StackTypes::I32(0));
                    return;
                }
                let mut cvec = Vec::new();
                for _ in 0..typ_args_len {
                    match self.value_stack.pop() {
                        Some(v) => cvec.push(v),
                        None => cvec.push(StackTypes::I32(0)),
                    }
                }
                cvec.reverse();
                let func = &self.module.fcce[local_idx];
                let fcode = &func.code;
                let mut vars = Vec::new();
                vars.extend(cvec);
                for (loc, typ) in &func.vars {
                    let ty = typ.as_ref().expect("CallIndirect vars type error");
                    let var = match ty {
                        TypeBytes::I32 => StackTypes::I32(0),
                        TypeBytes::I64 => StackTypes::I64(0),
                        TypeBytes::F32 => StackTypes::F32(0.0),
                        TypeBytes::F64 => StackTypes::F64(0.0),
                    };
                    for _ in 0..*loc {
                        vars.push(var.clone());
                    }
                }
                self.call_stack.push(StackCalls { fnid: local_idx, code: fcode.to_vec(), loc: 0, vars });
            },
            //Args
            Code::Drop =>
            {
                let waste = self.value_stack.pop();
                lstring = format!("{}. Drop {:?}", self.incount, waste);
            },
            Code::Select =>
            {
                let sel = match self.value_stack.pop(){
                    Some(StackTypes::I32(val)) => val,
                    _ => panic!("Stack Error Select"),
                };
                lstring = format!("{}. Select {}", self.incount, sel);
                let val2 = self.value_stack.pop().unwrap_or(StackTypes::I32(0));
                let val1 = self.value_stack.pop().unwrap_or(StackTypes::I32(0));

                if sel != 0 {self.value_stack.push(val1);}
                else{self.value_stack.push(val2);}
            },
            //Vars
            Code::LocalGet(loc) => {
                let val = call.vars.get(loc as usize)
                    .cloned()
                    .unwrap_or(StackTypes::I32(0));
                lstring = format!("{}. Local Get({}): {:?}", self.incount, loc, val);
                self.value_stack.push(val);
            },
            Code::LocalSet(loc) => {
                let to_stack = self.value_stack.pop().unwrap_or(StackTypes::I32(0));
                lstring = format!("{}. Local Set({}) {:?}", self.incount, loc, to_stack);
                let idx = loc as usize;
                if idx < call.vars.len() {
                    call.vars[idx] = to_stack;
                }
            },
            Code::LocalTee(loc) =>
            {
                let to_loc = match self.value_stack.last().cloned() {
                    Some(v) => v,
                    None => StackTypes::I32(0), // stack underflow — stub
                };
                let ind = loc as usize;
                if ind < call.vars.len() {
                    lstring = format!("{}. LocalTee({}) {:?}", self.incount, loc, to_loc);
                    call.vars[ind] = to_loc;
                }

            },
            Code::GlobalGet(loc) =>
            {
                let loc = loc as usize;
                // Imported globals may not be in our globs vec; use I32(0) as stub.
                let to_stack = if loc < self.globs.len() {
                    self.globs[loc].typ.clone()
                } else {
                    StackTypes::I32(0)
                };
                lstring = format!("{}. Global Get({}) {:?}", self.incount, loc, to_stack);
                self.value_stack.push(to_stack);
            },
            Code::GlobalSet(loc) =>
            {
                let to_glob = self.value_stack.pop().expect("Stack empty globset");
                let loc = loc as usize;
                if loc < self.globs.len() && self.globs[loc].ismut {
                    lstring = format!("{}. Global Set({}) {:?}", self.incount, loc, to_glob);
                    self.globs[loc].typ = to_glob;
                } else {
                    // Imported or immutable global — discard the value silently.
                    lstring = format!("{}. Global Set({}) (stub, ignored)", self.incount, loc);
                }
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
                lstring = format!("{}. I32Load({}) {}", self.incount, off, to_stack);
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
                lstring = format!("{}. I64Load({}) {}", self.incount, off, to_stack);
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
                lstring = format!("{}. F32Load({}) {}", self.incount, off, to_stack);
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
                lstring = format!("{}. F64Load({}) {}", self.incount, off, to_stack);
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
                lstring = format!("{}. I32Load8({}) {}", self.incount, off, val);
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
                lstring = format!("{}. I32Load8U({}) {}", self.incount, off, val);
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
                lstring = format!("{}. I32Load16S({}) {}", self.incount, off, val);
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
                lstring = format!("{}. I32Load16U({}) {}", self.incount, off, val);
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
                lstring = format!("{}. I64Load8S({}) {}", self.incount, off, val);
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
                lstring = format!("{}. I64Load8U({}) {}", self.incount, off, val);
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
                lstring = format!("{}. I64Load16S({}) {}", self.incount, off, val);
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
                lstring = format!("{}. I64Load16U({}) {}", self.incount, off, val);
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
                lstring = format!("{}. I64Load32S({}) {}", self.incount, off, val);
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
                lstring = format!("{}. I64Load32U({}) {}", self.incount, off, val);
                self.value_stack.push(StackTypes::I64(val as i64));
            },
            //STR
            Code::I32Store(off) =>
            {
                let var = self.pop_i32();
                let memloc = self.pop_u32();
                let uloc = (off + memloc) as usize;
                let bytes = var.to_le_bytes();
                lstring = format!("{}. I32Store({}) {:?}", self.incount, off, bytes);
                if uloc + 4 <= self.mem.len() {
                    self.mem[uloc..uloc + 4].copy_from_slice(&bytes);
                }
            },
            Code::I64Store(off) =>
            {
                let var = self.pop_i64();
                let memloc = self.pop_u32();
                let uloc = (off + memloc) as usize;
                let bytes = var.to_le_bytes();
                lstring = format!("{}. I64Store({}) {:?}", self.incount, off, bytes);
                if uloc + 8 <= self.mem.len() {
                    self.mem[uloc..uloc + 8].copy_from_slice(&bytes);
                }
            },
            Code::F32Store(off) =>
            {
                let var = self.pop_f32();
                let memloc = self.pop_u32();
                let uloc = (off + memloc) as usize;
                let bytes = var.to_le_bytes();
                lstring = format!("{}. F32Store({}) {:?}", self.incount, off, bytes);
                if uloc + 4 <= self.mem.len() {
                    self.mem[uloc..uloc + 4].copy_from_slice(&bytes);
                }
            },
            Code::F64Store(off) =>
            {
                let var = self.pop_f64();
                let memloc = self.pop_u32();
                let uloc = (off + memloc) as usize;
                let bytes = var.to_le_bytes();
                lstring = format!("{}. F64Store({}) {:?}", self.incount, off, bytes);
                if uloc + 8 <= self.mem.len() {
                    self.mem[uloc..uloc + 8].copy_from_slice(&bytes);
                }
            },
            Code::I32Store8(off) =>
            {
                let var = self.pop_i32() as u8;
                let memloc = self.pop_u32();
                let uloc = (off + memloc) as usize;
                lstring = format!("{}. I32Store8({}) {}", self.incount, off, var);
                if uloc < self.mem.len() {
                    self.mem[uloc] = var;
                }
            },
            Code::I32Store16(off) =>
            {
                let var = self.pop_i32() as u16;
                let memloc = self.pop_u32();
                let uloc = (off + memloc) as usize;
                lstring = format!("{}. I32Store16({}) {}", self.incount, off, var);
                if uloc + 2 <= self.mem.len() {
                    self.mem[uloc..uloc + 2].copy_from_slice(&var.to_le_bytes());
                }
            },
            Code::I64Store8(off) =>
            {
                let var = self.pop_i64() as u8;
                let memloc = self.pop_u32();
                let uloc = (off + memloc) as usize;
                lstring = format!("{}. I64Store8({}) {}", self.incount, off, var);
                if uloc < self.mem.len() {
                    self.mem[uloc] = var;
                }
            },
            Code::I64Store16(off) =>
            {
                let var = self.pop_i64() as u16;
                let memloc = self.pop_u32();
                let uloc = (off + memloc) as usize;
                lstring = format!("{}. I64Store16({}) {}", self.incount, off, var);
                if uloc + 2 <= self.mem.len() {
                    self.mem[uloc..uloc + 2].copy_from_slice(&var.to_le_bytes());
                }
            },
            Code::I64Store32(off) =>
            {
                let var = self.pop_i64() as u32;
                let memloc = self.pop_u32();
                let uloc = (off + memloc) as usize;
                lstring = format!("{}. I64Store32({}) {}", self.incount, off, var);
                if uloc + 4 <= self.mem.len() {
                    self.mem[uloc..uloc + 4].copy_from_slice(&var.to_le_bytes());
                }
            },
            Code::MemorySize => 
            {
                let memlen = self.mem.len();
                lstring = format!("{}. MemorySize {} ", self.incount, memlen);
                self.value_stack.push(StackTypes::I32((memlen/65536) as i32));
            },
            Code::MemoryGrow => 
            {
                let memchange = self.pop_i32();
                let curmem = (self.mem.len() / 65536) as i32;
                if memchange < 0 {
                    // Invalid grow request — return -1 per spec
                    lstring = format!("{}. MemoryGrow FAILED (negative delta {})", self.incount, memchange);
                    self.value_stack.push(StackTypes::I32(-1));
                } else {
                    let new_pages = (curmem + memchange) as u32;
                    let allowed = match self.memmax {
                        Some(max_pages) => new_pages <= max_pages,
                        None => true,
                    };
                    if allowed {
                        self.mem.resize((new_pages as usize) * 65536, 0);
                        lstring = format!("{}. MemoryGrow New: {} Old: {}", self.incount, new_pages, curmem);
                        self.value_stack.push(StackTypes::I32(curmem));
                    } else {
                        // Grow would exceed max — return -1 per spec
                        lstring = format!("{}. MemoryGrow FAILED (would exceed max)", self.incount);
                        self.value_stack.push(StackTypes::I32(-1));
                    }
                }
            },
            //Cons
            Code::I32Const(cons) => {
                lstring = format!("{}. I32Const {}", self.incount, cons);
                self.value_stack.push(StackTypes::I32(cons));
                //log::info!("I32 Constant: {}", cons);
            },
            Code::I64Const(cons) => {
                lstring = format!("{}. I64Const {}", self.incount, cons);
                self.value_stack.push(StackTypes::I64(cons));
                //log::info!("I64 Constant: {}", cons);
            },
            Code::F32Const(cons) => {
                lstring = format!("{}. F32Const {}", self.incount, cons);
                self.value_stack.push(StackTypes::F32(cons));
                //log::info!("F32 Constant: {}", cons);
            },
            Code::F64Const(cons) => {
                lstring = format!("{}. F64Const {}", self.incount, cons);
                self.value_stack.push(StackTypes::F64(cons));
                //log::info!("F64 Constant {}", cons);
            },
            //Comps
            //I32
            Code::I32Eqz => {
                let i_val = self.pop_i32();
                lstring = format!("{}. I32Eqz {}", self.incount, i_val);
                match i_val
                {
                    0 => self.value_stack.push(StackTypes::I32(1)),
                    _ => self.value_stack.push(StackTypes::I32(0)),
                }

            },
            Code::I32Eq => {
                let val2 = self.pop_i32();
                let val1 = self.pop_i32();
                lstring = format!("{}. I32Eq Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 == val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            Code::I32Ne => {
                let val2 = self.pop_i32();
                let val1 = self.pop_i32();
                lstring = format!("{}. I32Ne Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 != val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            Code::I32LtS => {
                let val2 = self.pop_i32();
                let val1 = self.pop_i32();
                lstring = format!("{}. I32LtS Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 < val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            Code::I32LtU => {
                let val2 = self.pop_u32();
                let val1 = self.pop_u32();
                lstring = format!("{}. I32LtU Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 < val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            Code::I32GtS => 
            {
                let val2 = self.pop_i32();
                let val1 = self.pop_i32();
                lstring = format!("{}. I32GtS Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 > val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            Code::I32GtU => {
                let val2 = self.pop_u32();
                let val1 = self.pop_u32();
                lstring = format!("{}. I32GtU Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 > val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            Code::I32LeS => {
                let val2 = self.pop_i32();
                let val1 = self.pop_i32();
                lstring = format!("{}. I32LeS Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 <= val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            Code::I32LeU => {
                let val2 = self.pop_u32();
                let val1 = self.pop_u32();
                lstring = format!("{}. I32LeU Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 <= val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            Code::I32GeS => {
                let val2 = self.pop_i32();
                let val1 = self.pop_i32();
                lstring = format!("{}. I32GeS Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 >= val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            Code::I32GeU => {
                let val2 = self.pop_u32();
                let val1 = self.pop_u32();
                lstring = format!("{}. I32GeU Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 >= val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            //I64
            Code::I64Eqz => {
                let val = self.pop_i64();
                lstring = format!("{}. I64Eqz {}", self.incount, val);
                if val == 0 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            Code::I64Eq => {
                let val2 = self.pop_i64();
                let val1 = self.pop_i64();
                lstring = format!("{}. I64Eq Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 == val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            Code::I64Ne => {
                let val2 = self.pop_i64();
                let val1 = self.pop_i64();
                lstring = format!("{}. I64Ne Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. I64LtS Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. I64LtU Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 < val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            Code::I64GtS => {
                let val2 = self.pop_i64();
                let val1 = self.pop_i64();
                lstring = format!("{}. I64GtS Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. I64GtU Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 > val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            Code::I64LeS => {
                let val2 = self.pop_i64();
                let val1 = self.pop_i64();
                lstring = format!("{}. I64LeS Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. I64LeU Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 <= val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            Code::I64GeS => {
                let val2 = self.pop_i64();
                let val1 = self.pop_i64();
                lstring = format!("{}. I64GeS Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. I64GeU Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 >= val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            //F32
            Code::F32Eq => {
                let val2 = self.pop_f32();
                let val1 = self.pop_f32();
                lstring = format!("{}. F32Eq Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 == val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            Code::F32Ne => {
                let val2 = self.pop_f32();
                let val1 = self.pop_f32();
                lstring = format!("{}. F32Ne Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 != val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            Code::F32Lt => {
                let val2 = self.pop_f32();
                let val1 = self.pop_f32();
                lstring = format!("{}. F32Lt Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 < val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            Code::F32Gt => {
                let val2 = self.pop_f32();
                let val1 = self.pop_f32();
                lstring = format!("{}. F32Gt Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 > val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            Code::F32Le => {
                let val2 = self.pop_f32();
                let val1 = self.pop_f32();
                lstring = format!("{}. F32Le Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 <= val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
                //Calcs
            //I32
//                Code::I32Clz => (),
            Code::I32Clz => {
                let val = self.pop_i32();
                let leading_zeros = val.leading_zeros();
                lstring = format!("{}. I32Clz {}", self.incount, val);
                self.value_stack.push(StackTypes::I32(leading_zeros as i32));
            },
//               Code::I32Ctz => (),
            Code::I32Ctz => {
                let val = self.pop_i32();
                let trailing_zeros = val.trailing_zeros();
                lstring = format!("{}. I32Ctz {}", self.incount, val);
                self.value_stack.push(StackTypes::I32(trailing_zeros as i32));
            },  
//                Code::I32Popcnt => (),
            Code::I32Popcnt => {
                let val = self.pop_i32();
                lstring = format!("{}. I32Popcnt {}", self.incount, val);
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
                lstring = format!("{}. I32Add Val1: {} Val2: {}", self.incount, val1, val2);
                self.value_stack.push(StackTypes::I32(val1.wrapping_add(val2)));
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
                lstring = format!("{}. I32Sub Val1: {} Val2: {}", self.incount, val1, val2);
                self.value_stack.push(StackTypes::I32(val1.wrapping_sub(val2)));
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
                lstring = format!("{}. I32Mul Val1: {} Val2: {}", self.incount, val1, val2);
                self.value_stack.push(StackTypes::I32(val1.wrapping_mul(val2)));
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
                lstring = format!("{}. I32DivS Val1: {}/ Val2: {}", self.incount, val1, val2);
                let result = if val2 == 0 { 0 } else { val1.wrapping_div(val2) };
                self.value_stack.push(StackTypes::I32(result));
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
            lstring = format!("{}. I32DivU Val1: {}/ Val2: {}", self.incount, val1, val2);
            let result = if val2 == 0 { 0u32 } else { val1 / val2 };
            self.value_stack.push(StackTypes::I32(result as i32));
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
                lstring = format!("{}. I32RemS Val1: {} Val2: {}", self.incount, a, b);
            let result = if b == 0 { 0 } else { a % b };
            self.value_stack.push(StackTypes::I32(result));
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
                lstring = format!("{}. I32RemU Val1: {} Val2: {}", self.incount, a, b);
                // WASM trap on divide-by-zero; return 0 as best-effort stub
                let result = if b == 0 { 0u32 } else { a % b };
                self.value_stack.push(StackTypes::I32(result as i32));
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
                lstring = format!("{}. I32And Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. I32Or Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. I32Xor Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. I32Shl Val1: {} Val2: {}", self.incount, a, b);
                self.value_stack.push(StackTypes::I32(a.wrapping_shl(b)));
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
                lstring = format!("{}. I32ShrS Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. I32ShrU Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. I32Rotl Shift: {} Val: {}", self.incount, shift, value);
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
                lstring = format!("{}. I32Rotr Shift: {} Value: {}", self.incount, shift, value);
                self.value_stack.push(StackTypes::I32(value.rotate_right(shift) as i32));
            },
                            //I64
            //                Code::I64Clz => (),
            Code::I64Clz => {
                let value = match self.value_stack.pop() {
                    Some(StackTypes::I64(v)) => v,
                    _ => panic!("I64Clz error"),
                };
                lstring = format!("{}. I64Clz Value: {}", self.incount, value);
                self.value_stack.push(StackTypes::I64(value.leading_zeros() as i64));
            },
            //                Code::I64Ctz => (),
            Code::I64Ctz => {
                let value = match self.value_stack.pop() {
                    Some(StackTypes::I64(v)) => v,
                    _ => panic!("I64Ctz error"),
                };
                lstring = format!("{}. I64Ctz Value: {}", self.incount, value);
                self.value_stack.push(StackTypes::I64(value.trailing_zeros() as i64));
            },
            //                Code::I64Popcnt => (),
            Code::I64Popcnt => {
                let value = match self.value_stack.pop() {
                    Some(StackTypes::I64(v)) => v,
                    _ => panic!("I64Popcnt error"),
                };
                lstring = format!("{}. I64Popcnt Value: {}", self.incount, value);
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
                lstring = format!("{}. I64Add Val1: {} Val2: {}", self.incount, a, b);
                self.value_stack.push(StackTypes::I64(a.wrapping_add(b)));
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
                lstring = format!("{}. I64Sub Val1: {} Val2: {}", self.incount, a, b);
                self.value_stack.push(StackTypes::I64(a.wrapping_sub(b)));
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
                lstring = format!("{}. I64Mul Val1: {} Val2: {}", self.incount, a, b);
                self.value_stack.push(StackTypes::I64(a.wrapping_mul(b)));
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
                lstring = format!("{}. I64DivS Val1: {} Val2: {}", self.incount, a, b);
                let result = if b == 0 { 0i64 } else { a.wrapping_div(b) };
                self.value_stack.push(StackTypes::I64(result));
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
                lstring = format!("{}. I64DivU Val1: {} Val2: {}", self.incount, a, b);
                let result = if b == 0 { 0u64 } else { a / b };
                self.value_stack.push(StackTypes::I64(result as i64));
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
                lstring = format!("{}. I64RemS Val1: {} Val2: {}", self.incount, a, b);
                let result = if b == 0 { 0i64 } else { a % b };
                self.value_stack.push(StackTypes::I64(result));
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
                lstring = format!("{}. I64RemU Val1: {} Val2: {}", self.incount, a, b);
                let result = if b == 0 { 0u64 } else { a % b };
                self.value_stack.push(StackTypes::I64(result as i64));
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
                lstring = format!("{}. I64And Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. I64Or Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. I64Xor Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. I64Shl Shift: {} Val: {}", self.incount, shift, value);
                self.value_stack.push(StackTypes::I64(value.wrapping_shl(shift as u32)));
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
                lstring = format!("{}. I64ShrS Shift: {} Val: {}", self.incount, shift, value);
                self.value_stack.push(StackTypes::I64(value.wrapping_shr(shift as u32)));
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
                lstring = format!("{}. I64 Shift: {} Value: {}", self.incount, shift, value);
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
                lstring = format!("{}. I64Rotl Shift: {} value: {}", self.incount, shift, value);
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
                lstring = format!("{}. I64Rotr Shift: {} Value: {}", self.incount, shift, value);
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
                lstring = format!("{}. F32Abs Val: {}", self.incount, value);
                self.value_stack.push(StackTypes::F32(value.abs()));
            },
            //                Code::F32Neg => (),
            Code::F32Neg => {
                let value = match self.value_stack.pop()
                {
                    Some(StackTypes::F32(v)) => v,
                    _ => panic!("F32Neg error"),
                };
                lstring = format!("{}. F32Neg Val: {}", self.incount, value);
                self.value_stack.push(StackTypes::F32(-value));
            },
            //                Code::F32Ceil => (),
            Code::F32Ceil => {
                let value = match self.value_stack.pop()
                {
                    Some(StackTypes::F32(v)) => v,
                    _ => panic!("F32Ceil error"),
                };
                lstring = format!("{}. F32Ceil Val: {}", self.incount, value);
                self.value_stack.push(StackTypes::F32(value.ceil()));
            },
            //                Code::F32Floor => (),
            Code::F32Floor => {
                let value = match self.value_stack.pop()
                {
                    Some(StackTypes::F32(v)) => v,
                    _ => panic!("F32Floor error"),
                };
                lstring = format!("{}. F32Floor Val: {}", self.incount, value);
                self.value_stack.push(StackTypes::F32(value.floor()));
            },
            //                Code::F32Trunc => (),
            Code::F32Trunc => {
                let value = match self.value_stack.pop()
                {
                    Some(StackTypes::F32(v)) => v,
                    _ => panic!("F32Trunc error"),
                };
                lstring = format!("{}. F32Trunc Val: {}", self.incount, value);
                self.value_stack.push(StackTypes::F32(value.trunc()));
            },
            //                Code::F32Nearest => (),
            Code::F32Nearest => {
                let value = match self.value_stack.pop()
                {
                    Some(StackTypes::F32(v)) => v,
                    _ => panic!("F32Nearest error"),
                };
                lstring = format!("{}. F32Nearest Val: {}", self.incount, value);
                self.value_stack.push(StackTypes::F32(value.round()));
            },
            //                Code::F32Sqrt => (),
            Code::F32Sqrt => {
                let value = match self.value_stack.pop()
                {
                    Some(StackTypes::F32(v)) => v,
                    _ => panic!("F32Sqrt error"),
                };
                lstring = format!("{}. F32Sqrt Val: {}", self.incount, value);
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
                lstring = format!("{}. F32Add Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. F32Sub Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. F32Mul Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. F32Div Val1: {}/ Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. F32Min Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. F32Max Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. F32Copysign Sign: {} Value: {}", self.incount, sign, value);
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
                lstring = format!("{}. F64Abs Value: {}", self.incount, v);
                self.value_stack.push(StackTypes::F64(v.abs()));
            },
            //                Code::F64Neg => (),
            Code::F64Neg => {
                let v = match self.value_stack.pop()
                {
                    Some(StackTypes::F64(v)) => v,
                    _ => panic!("F64Neg error"),
                };
                lstring = format!("{}. F64Neg Value: {}", self.incount, v);
                self.value_stack.push(StackTypes::F64(-v));
            },
            //                Code::F64Ceil => (),
            Code::F64Ceil => {
                let v = match self.value_stack.pop()
                {
                    Some(StackTypes::F64(v)) => v,
                    _ => panic!("F64Ceil error"),
                };
                lstring = format!("{}. F64Ceil Value: {}", self.incount, v);
                self.value_stack.push(StackTypes::F64(v.ceil()));
            },
            //                Code::F64Floor => (),
            Code::F64Floor => {
                let v = match self.value_stack.pop()
                {
                    Some(StackTypes::F64(v)) => v,
                    _ => panic!("F64Floor error"),
                };
                lstring = format!("{}. F64Floor Value: {}", self.incount, v);
                self.value_stack.push(StackTypes::F64(v.floor()));
            },
            //                Code::F64Trunc => (),
            Code::F64Trunc => {
                let v = match self.value_stack.pop()
                {
                    Some(StackTypes::F64(v)) => v,
                    _ => panic!("F64Trunc error"),
                };
                lstring = format!("{}. F64Trunc Value: {}", self.incount, v);
                self.value_stack.push(StackTypes::F64(v.trunc()));
            },
            //                Code::F64Nearest => (),
            Code::F64Nearest => {
                let v = match self.value_stack.pop()
                {
                    Some(StackTypes::F64(v)) => v,
                    _ => panic!("F64Nearest error"),
                };
                lstring = format!("{}. F64Nearest Value: {}", self.incount, v);
                self.value_stack.push(StackTypes::F64(v.round()));
            },
            //                Code::F64Sqrt => (),
            Code::F64Sqrt => {
                let v = match self.value_stack.pop()
                {
                    Some(StackTypes::F64(v)) => v,
                    _ => panic!("F64Sqrt error"),
                };
                lstring = format!("{}. F64Sqrt Value: {}", self.incount, v);
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
                lstring = format!("{}. F64Add Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. F64Sub Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. F64Mul Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. F64Div Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. F64Min Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. F64Max Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. F64Copysign Sign: {} Value: {}", self.incount, sign, value);
                self.value_stack.push(StackTypes::F64(sign.copysign(value)));
            },
            //tools
            Code::F32Ge => {
                let val2 = self.pop_f32();
                let val1 = self.pop_f32();
                lstring = format!("{}. F64Ge Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 >= val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            //F64
            Code::F64Eq => {
                let val2 = self.pop_f64();
                let val1 = self.pop_f64();
                lstring = format!("{}. F64eq Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 == val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            Code::F64Ne => {
                let val2 = self.pop_f64();
                let val1 = self.pop_f64();
                lstring = format!("{}. F64Ne Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 != val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            Code::F64Lt => {
                let val2 = self.pop_f64();
                let val1 = self.pop_f64();
                lstring = format!("{}. F64Lt Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 < val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            Code::F64Gt => {
                let val2 = self.pop_f64();
                let val1 = self.pop_f64();
                lstring = format!("{}. F64Gt Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 > val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            Code::F64Le => {
                let val2 = self.pop_f64();
                let val1 = self.pop_f64();
                lstring = format!("{}. F64Le Val1: {} Val2: {}", self.incount, val1, val2);
                if val1 <= val2 {self.value_stack.push(StackTypes::I32(1));}
                else {self.value_stack.push(StackTypes::I32(0));}
            },
            Code::F64Ge => {
                let val2 = self.pop_f64();
                let val1 = self.pop_f64();
                lstring = format!("{}. F64Ge Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. I32WrapI64 Value: {}", self.incount, wrapped);
                self.value_stack.push(StackTypes::I32(wrapped));
            },
            Code::I32TruncF32S => 
            {
                let trunced = match self.value_stack.pop()
                {
                    Some(StackTypes::F32(val)) => val as i32,
                    _ => panic!("Invalid Stack Type I32WrapF32S"),
                };
                lstring = format!("{}. I32TruncF32S Value: {}", self.incount, trunced);
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
                lstring = format!("{}. I32TruncF32U Value: {}", self.incount, trunced);
                self.value_stack.push(StackTypes::I32(sender));
            },
            Code::I32TruncF64S => 
            {
                let trunced = match self.value_stack.pop()
                {
                    Some(StackTypes::F64(val)) => val as i32,
                    _ => panic!("Stack type is not a F64 I32TruncF64S"),
                };
                lstring = format!("{}. I32TruncF64S Value: {}", self.incount, trunced);
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
                lstring = format!("{}. I32TruncF64U Value: {}", self.incount, trunced);
                self.value_stack.push(StackTypes::I32(trunced as i32));
            },
            Code::I64ExtendI32S => 
            {
                let extend = match self.value_stack.pop()
                {
                    Some(StackTypes::I32(val)) => val as i64,
                    _ => panic!("Stack type is not I32 I64extendI32S"),
                };
                lstring = format!("{}. I64ExtendI32S Value: {}", self.incount, extend);
                self.value_stack.push(StackTypes::I64(extend));
            },  
            Code::I64ExtendI32U => 
            {
                let val = self.pop_i32();
                // Reinterpret as unsigned 32-bit, then zero-extend to 64-bit
                let extend = val as u32 as u64;
                lstring = format!("{}. I64ExtendI32U Value: {}", self.incount, extend);
                self.value_stack.push(StackTypes::I64(extend as i64));
            },
            Code::I64TruncF32S => 
            {
                let trunced = match self.value_stack.pop()
                {
                    Some(StackTypes::F32(val)) => val as i64,
                    _ => panic!("Stack type is not F32 I64TruncF32S"),
                };
                lstring = format!("{}. I64TruncF32S Value: {}", self.incount, trunced);
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
                lstring = format!("{}. I64TruncF32U Value: {}", self.incount, trunced);
                self.value_stack.push(StackTypes::I64(trunced as i64));
            },
            Code::I64TruncF64S => 
            {
                let trunced = match self.value_stack.pop()
                {
                    Some(StackTypes::F64(val)) => val as i64,
                    _ => panic!("Stack type is not F64 I64TruncF64S"),
                };
                lstring = format!("{}. I64TruncF64S Value: {}", self.incount, trunced);
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
                lstring = format!("{}. I64TruncF64U Value: {}", self.incount, trunced);
                self.value_stack.push(StackTypes::I64(trunced as i64));
            },
            Code::F32ConvertI32S => 
            {
                let converted = match self.value_stack.pop()
                {
                    Some(StackTypes::I32(val)) => val as f32,
                    _ => panic!("Stack type is not I32 F32ConvertI32S"),
                };
                lstring = format!("{}. F32ConvertI32S Value: {}", self.incount, converted);
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
                lstring = format!("{}. F32ConvertI32U Value: {}", self.incount, converted);
                self.value_stack.push(StackTypes::F32(converted));
            },
            Code::F32ConvertI64S => 
            {
                let converted = match self.value_stack.pop()
                {
                    Some(StackTypes::I64(val)) => val as f32,
                    _ => panic!("Stack type not I64 F32ConvertI64S"),
                };
                lstring = format!("{}. F32ConvertI64S Value: {}", self.incount, converted);
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
                lstring = format!("{}. F32ConvertI64U Value: {}", self.incount, converted);
                self.value_stack.push(StackTypes::F32(converted));
            },
            Code::F32DemoteF64 => 
            {
                match self.value_stack.pop()
                {
                    Some(StackTypes::F64(val)) => {
                        lstring = format!("{}. F32DemoteF64 Value: {}", self.incount, val);
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
                        lstring = format!("{}. F64ConvertI32S Value: {}", self.incount, val);
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
                        lstring = format!("{}. ConvertI32U Value: {}", self.incount, val);
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
                        lstring = format!("{}. F64ConvertI64S Value: {}", self.incount, val);
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
                        lstring = format!("{}. F64ConvertI64U Value: {}", self.incount, val);
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
                        lstring = format!("{}. F64PromoteF32 Value: {}", self.incount, val);
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
                        lstring = format!("{}. I64ReinterpretF64 Value: {}", self.incount, val);
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
                        lstring = format!("{}. I32ReinterpretF32 Value: {}", self.incount, val);
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
                        lstring = format!("{}. F64ReinterpretI64 Value: {}", self.incount, val);
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
                        lstring = format!("{}. F32ReinterpretI32 Value: {}", self.incount, val);
                        self.value_stack.push(StackTypes::F32(f32::from_bits(val as u32)));
                    }
                    _ => panic!("Stack type not I32 F32ReinterpretI32"),
                }
            },
            // All OP codes should be implemented now.
            // --- Sign-extension ops ---
            Code::I32Extend8S => {
                let v = self.pop_i32();
                self.value_stack.push(StackTypes::I32((v as i8) as i32));
                lstring = format!("{}.  I32Extend8S", self.incount);
            },
            Code::I32Extend16S => {
                let v = self.pop_i32();
                self.value_stack.push(StackTypes::I32((v as i16) as i32));
                lstring = format!("{}.  I32Extend16S", self.incount);
            },
            Code::I64Extend8S => {
                let v = self.pop_i64();
                self.value_stack.push(StackTypes::I64((v as i8) as i64));
                lstring = format!("{}.  I64Extend8S", self.incount);
            },
            Code::I64Extend16S => {
                let v = self.pop_i64();
                self.value_stack.push(StackTypes::I64((v as i16) as i64));
                lstring = format!("{}.  I64Extend16S", self.incount);
            },
            Code::I64Extend32S => {
                let v = self.pop_i64();
                self.value_stack.push(StackTypes::I64((v as i32) as i64));
                lstring = format!("{}.  I64Extend32S", self.incount);
            },
            // --- Reference type ops (stub: push null/false/0) ---
            Code::RefNull => {
                self.value_stack.push(StackTypes::I32(0)); // null ref represented as 0
                lstring = format!("{}.  RefNull", self.incount);
            },
            Code::RefIsNull => {
                let v = match self.value_stack.pop() {
                    Some(StackTypes::I32(v)) => if v == 0 { 1 } else { 0 },
                    _ => 1,
                };
                self.value_stack.push(StackTypes::I32(v));
                lstring = format!("{}.  RefIsNull", self.incount);
            },
            Code::RefFunc(_idx) => {
                self.value_stack.push(StackTypes::I32(0)); // stub: push null funcref
                lstring = format!("{}.  RefFunc", self.incount);
            },
            // --- Misc/saturating ops (0xFC prefix) ---
            Code::MiscOp(sub) => {
                match sub {
                    // i32.trunc_sat_f32_s — NaN→0, clamp to [i32::MIN, i32::MAX]
                    0 => {
                        let result = match self.value_stack.pop() {
                            Some(StackTypes::F32(v)) => {
                                if v.is_nan() { 0i32 }
                                else if v >= i32::MAX as f32 { i32::MAX }
                                else if v <= i32::MIN as f32 { i32::MIN }
                                else { v as i32 }
                            },
                            _ => 0,
                        };
                        lstring = format!("{}.  I32TruncSatF32S {}", self.incount, result);
                        self.value_stack.push(StackTypes::I32(result));
                    },
                    // i32.trunc_sat_f32_u — NaN/neg→0, clamp to [0, u32::MAX]
                    1 => {
                        let result = match self.value_stack.pop() {
                            Some(StackTypes::F32(v)) => {
                                if v.is_nan() || v < 0.0 { 0u32 }
                                else if v >= u32::MAX as f32 { u32::MAX }
                                else { v as u32 }
                            },
                            _ => 0,
                        };
                        lstring = format!("{}.  I32TruncSatF32U {}", self.incount, result);
                        self.value_stack.push(StackTypes::I32(result as i32));
                    },
                    // i32.trunc_sat_f64_s
                    2 => {
                        let result = match self.value_stack.pop() {
                            Some(StackTypes::F64(v)) => {
                                if v.is_nan() { 0i32 }
                                else if v >= i32::MAX as f64 { i32::MAX }
                                else if v <= i32::MIN as f64 { i32::MIN }
                                else { v as i32 }
                            },
                            _ => 0,
                        };
                        lstring = format!("{}.  I32TruncSatF64S {}", self.incount, result);
                        self.value_stack.push(StackTypes::I32(result));
                    },
                    // i32.trunc_sat_f64_u
                    3 => {
                        let result = match self.value_stack.pop() {
                            Some(StackTypes::F64(v)) => {
                                if v.is_nan() || v < 0.0 { 0u32 }
                                else if v >= u32::MAX as f64 { u32::MAX }
                                else { v as u32 }
                            },
                            _ => 0,
                        };
                        lstring = format!("{}.  I32TruncSatF64U {}", self.incount, result);
                        self.value_stack.push(StackTypes::I32(result as i32));
                    },
                    // i64.trunc_sat_f32_s
                    4 => {
                        let result = match self.value_stack.pop() {
                            Some(StackTypes::F32(v)) => {
                                if v.is_nan() { 0i64 }
                                else if v >= i64::MAX as f32 { i64::MAX }
                                else if v <= i64::MIN as f32 { i64::MIN }
                                else { v as i64 }
                            },
                            _ => 0,
                        };
                        lstring = format!("{}.  I64TruncSatF32S {}", self.incount, result);
                        self.value_stack.push(StackTypes::I64(result));
                    },
                    // i64.trunc_sat_f32_u
                    5 => {
                        let result = match self.value_stack.pop() {
                            Some(StackTypes::F32(v)) => {
                                if v.is_nan() || v < 0.0 { 0u64 }
                                else if v >= u64::MAX as f32 { u64::MAX }
                                else { v as u64 }
                            },
                            _ => 0,
                        };
                        lstring = format!("{}.  I64TruncSatF32U {}", self.incount, result);
                        self.value_stack.push(StackTypes::I64(result as i64));
                    },
                    // i64.trunc_sat_f64_s
                    6 => {
                        let result = match self.value_stack.pop() {
                            Some(StackTypes::F64(v)) => {
                                if v.is_nan() { 0i64 }
                                else if v >= i64::MAX as f64 { i64::MAX }
                                else if v <= i64::MIN as f64 { i64::MIN }
                                else { v as i64 }
                            },
                            _ => 0,
                        };
                        lstring = format!("{}.  I64TruncSatF64S {}", self.incount, result);
                        self.value_stack.push(StackTypes::I64(result));
                    },
                    // i64.trunc_sat_f64_u
                    7 => {
                        let result = match self.value_stack.pop() {
                            Some(StackTypes::F64(v)) => {
                                if v.is_nan() || v < 0.0 { 0u64 }
                                else if v >= u64::MAX as f64 { u64::MAX }
                                else { v as u64 }
                            },
                            _ => 0,
                        };
                        lstring = format!("{}.  I64TruncSatF64U {}", self.incount, result);
                        self.value_stack.push(StackTypes::I64(result as i64));
                    },
                    // memory.init / memory.copy / memory.fill / table.init / table.copy / table.fill
                    // All pop 3 i32 operands; no result pushed (stub — no real memory/table impl)
                    8 | 10 | 11 | 12 | 14 | 17 => {
                        let _ = self.value_stack.pop();
                        let _ = self.value_stack.pop();
                        let _ = self.value_stack.pop();
                        lstring = format!("{}.  MiscOp({})[bulk-3]", self.incount, sub);
                    },
                    // data.drop / elem.drop — no stack effect
                    9 | 13 => {
                        lstring = format!("{}.  MiscOp({})[drop]", self.incount, sub);
                    },
                    // table.grow — pops (ref, i32), pushes i32 (-1 = out of memory stub)
                    15 => {
                        let _ = self.value_stack.pop();
                        let _ = self.value_stack.pop();
                        self.value_stack.push(StackTypes::I32(-1));
                        lstring = format!("{}.  TableGrow(stub=-1)", self.incount);
                    },
                    // table.size — pushes i32 (0 stub)
                    16 => {
                        self.value_stack.push(StackTypes::I32(0));
                        lstring = format!("{}.  TableSize(stub=0)", self.incount);
                    },
                    _ => {
                        lstring = format!("{}.  MiscOp({})[unknown]", self.incount, sub);
                    }
                }
            },
        }
        self.alogger(lstring);
        /*if call.loc >= call.code.len()
        {
            let turn = self.value_stack.pop();
            self.call_stack.pop();
            if self.call_stack.is_empty()
            {
                self.ended = true;
                return;
            }  
            if let Some(turner) = turn {self.value_stack.push(turner)}
        }*/
        self.incount += 1;
        //wasfile.flush().expect("Cant flush log file");
    }
}
