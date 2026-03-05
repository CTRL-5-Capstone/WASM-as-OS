use core::panic;
use super::wasm_module::*;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use serde::{Serialize, Deserialize};
use std::process::{Command, ChildStdin, Stdio};
use std::sync::{Arc, Mutex};
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
pub struct PFlags
{
    //I32
    pub i32_eqz: bool,
    pub i32_eq: bool,
    pub i32_ne: bool,
   //flow
    pub unreachable: bool,
    pub nop: bool,
    pub block: bool,
    pub pool: bool,
    pub fi: bool,
    pub esle: bool,
    pub end: bool,
    pub br: bool,
    pub br_if: bool,
    pub br_table: bool,
    pub nruter: bool,
    pub call: bool,
    pub call_indirect: bool,
    //Args
    pub drop: bool,
    pub select: bool,
    //Vars
    pub local_get: bool,
    pub local_set: bool,
    pub local_tee: bool,
    pub global_get: bool,
    pub global_set: bool,
 
    //Mem
    //LD
    pub i32_load: bool,
    pub i64_load: bool,
    pub f32_load: bool,
    pub f64_load: bool,
    //I32
    pub i32_load_8s: bool,
    pub i32_load_8u: bool,
    pub i32_load_16s: bool,
    pub i32_load_16u: bool,
    //I64
    pub i64_load_8s: bool,
    pub i64_load_8u: bool,
    pub i64_load_16s: bool,
    pub i64_load_16u: bool,
    pub i64_load_32s: bool,
    pub i64_load_32u: bool,
    //STR
    pub i32_store: bool,
    pub i64_store: bool,
    pub f32_store: bool,
    pub f64_store: bool,
    pub i32_store_8: bool,
    pub i32_store_16: bool,
    pub i64_store_8: bool,
    pub i64_store_16: bool,
    pub i64_store_32: bool,
    pub memory_size: bool,
    pub memory_grow: bool,
    //Cons
    pub i32_const: bool,
    pub i64_const: bool,
    pub f32_const: bool,
    pub f64_const: bool,
    //Comps    
    pub i32_lts: bool,
    pub i32_ltu: bool,
    pub i32_gts: bool,
    pub i32_gtu: bool,
    pub i32_les: bool,
    pub i32_leu: bool,
    pub i32_ges: bool,
    pub i32_geu: bool,
    //I64
    pub i64_eqz: bool,
    pub i64_eq: bool,
    pub i64_ne: bool,
    pub i64_lts: bool,
    pub i64_ltu: bool,
    pub i64_gts: bool,
    pub i64_gtu: bool,
    pub i64_les: bool,
    pub i64_leu: bool,
    pub i64_ges: bool,
    pub i64_geu: bool,
    //F32
    pub f32_wq: bool,
    pub f32_ne: bool,
    pub f32_lt: bool,
    pub f32_gt: bool,
    pub f32_le: bool,
    pub f32_ge: bool,
    //F64
    pub f64_eq: bool,
    pub f64_ne: bool,
    pub f64_lt: bool,
    pub f64_gt: bool,
    pub f64_le: bool,
    pub f64_ge: bool,
    //Calcs
    //I32
    pub i32_clz: bool,
    pub i32_ctz: bool,
    pub i32_popcnt: bool,
    pub i32_add: bool,
    pub i32_sub: bool,
    pub i32_mul: bool,
    pub i32_divs: bool,
    pub i32_divu: bool,
    pub i32_rems: bool,
    pub i32_remu: bool,
    pub i32_and: bool,
    pub i32_or: bool,
    pub i32_xor: bool,
    pub i32_shl: bool,
    pub i32_shrs: bool,
    pub i32_shru: bool,
    pub i32_rotl: bool,
    pub i32_rotr: bool,
    //I64
    pub i64_clz: bool,
    pub i64_ctz: bool,
    pub i64_popcnt: bool,
    pub i64_add: bool,
    pub i64_sub: bool,
    pub i64_mul: bool,
    pub i64_divs: bool,
    pub i64_divu: bool,
    pub i64_rems: bool,
    pub i64_remu: bool,
    pub i64_and: bool,
    pub i64_or: bool,
    pub i64_xor: bool,
    pub i64_shl: bool,
    pub i64_shrs: bool,
    pub i64_shru: bool,
    pub i64_rotl: bool,
    pub i64_rotr: bool,
    //FL
    //F32
    pub f32_abs: bool,
    pub f32_neg: bool,
    pub f32_ceil: bool,
    pub f32_floor: bool,
    pub f32_trunc: bool,
    pub f32_nearest: bool,
    pub f32_sqrt: bool,
    pub f32_add: bool,
    pub f32_sub: bool,
    pub f32_mul: bool,
    pub f32_div: bool,
    pub f32_min: bool,
    pub f32_max: bool,
    pub f32_copysign: bool,
    //F64
    pub f64_abs: bool,
    pub f64_neg: bool,
    pub f64_ceil: bool,
    pub f64_floor: bool,
    pub f64_trunc: bool,
    pub f64_nearest: bool,
    pub f64_sqrt: bool,
    pub f64_add: bool,
    pub f64_sub: bool,
    pub f64_mul: bool,
    pub f64_div: bool,
    pub f64_min: bool,
    pub f64_max: bool,
    pub f64_copysign: bool,
    //tools
    pub i32_wrap_i64: bool,
    pub i32_trunc_f32s: bool,
    pub i32_trunc_f32u: bool,
    pub i32_trunc_f64s: bool,
    pub i32_trunc_f64u: bool,
    pub i64_extend_i32s: bool,
    pub i64_extend_i32u: bool,
    pub i64_trunc_f32s: bool,
    pub i64_trunc_f32u: bool,
    pub i64_trunc_f64s: bool,
    pub i64_trunc_f64u: bool,
    pub f32_convert_i32s: bool,
    pub f32_convert_i32u: bool,
    pub f32_convert_i64s: bool,
    pub f32_convert_i64u: bool,
    pub f32_demote_f64: bool,
    pub f64_convert_i32s: bool,
    pub f64_converti_32u: bool,
    pub f64_convert_i64s: bool,
    pub f64_convert_i64u: bool,
    pub f64_promote_f32: bool,
    pub i32_reinterpret_f32: bool,
    pub i64_reinterpret_f64: bool,
    pub f32_reinterpret_i32: bool,
    pub f64_reinterpret_i64: bool
}
impl PFlags
{
    fn all_true(&mut self)
    {
        //I32
        self.i32_eqz = true;
        self.i32_eq = true;
        self.i32_ne = true;
        //flow
        self.unreachable = true;
        self.nop = true;
        self.block = true;
        self.pool = true;
        self.fi = true;
        self.esle = true;
        self.end = true;
        self.br = true;
        self.br_if = true;
        self.br_table = true;
        self.nruter = true;
        self.call = true;
        self.call_indirect = true;
        //Args
        self.drop = true;
        self.select = true;
        //Vars
        self.local_get = true;
        self.local_set = true;
        self.local_tee = true;
        self.global_get = true;
        self.global_set = true;

        //Mem
        //LD
        self.i32_load = true;
        self.i64_load = true;
        self.f32_load = true;
        self.f64_load = true;
        //I32
        self.i32_load_8s = true;
        self.i32_load_8u = true; 
        self.i32_load_16s = true;
        self.i32_load_16u = true;
        //I64
        self.i64_load_8s = true;
        self.i64_load_8u = true;
        self.i64_load_16s = true;
        self.i64_load_16u = true;
        self.i64_load_32s = true;
        self.i64_load_32u = true;
        //STR
        self.i32_store = true;
        self.i64_store = true;
        self.f32_store = true;
        self.f64_store = true;
        self.i32_store_8 = true;
        self.i32_store_16 = true;
        self.i64_store_8 = true;
        self.i64_store_16 = true;
        self.i64_store_32 = true;
        self.memory_size = true;
        self.memory_grow = true;
        //Cons
        self.i32_const = true;
        self.i64_const = true;
        self.f32_const = true;
        self.f64_const = true;
        //Comps    
        self.i32_lts = true;
        self.i32_ltu = true;
        self.i32_gts = true;
        self.i32_gtu = true;
        self.i32_les = true;
        self.i32_leu = true;
        self.i32_ges = true;
        self.i32_geu = true;
        //I64
        self.i64_eqz = true;
        self.i64_eq = true;
        self.i64_ne = true;
        self.i64_lts = true;
        self.i64_ltu = true;
        self.i64_gts = true;
        self.i64_gtu = true;
        self.i64_les = true;
        self.i64_leu = true;
        self.i64_ges = true;
        self.i64_geu = true;
        //F32
        self.f32_wq = true;
        self.f32_ne = true;
        self.f32_lt = true;
        self.f32_gt = true;
        self.f32_le = true;
        self.f32_ge = true;
        //F64
        self.f64_eq = true;
        self.f64_ne = true;
        self.f64_lt = true;
        self.f64_gt = true;
        self.f64_le = true;
        self.f64_ge = true;
        //Calcs
        //I32
        self.i32_clz = true;
        self.i32_ctz = true;
        self.i32_popcnt = true;
        self.i32_add = true;
        self.i32_sub = true;
        self.i32_mul = true;
        self.i32_divs = true;
        self.i32_divu = true;
        self.i32_rems = true;
        self.i32_remu = true;
        self.i32_and = true;
        self.i32_or = true;
        self.i32_xor = true;
        self.i32_shl = true;
        self.i32_shrs = true;
        self.i32_shru = true;
        self.i32_rotl = true;
        self.i32_rotr = true;
        //I64
        self.i64_clz = true;
        self.i64_ctz = true;
        self.i64_popcnt = true;
        self.i64_add = true;
        self.i64_sub = true;
        self.i64_mul = true;
        self.i64_divs = true;
        self.i64_divu = true;
        self.i64_rems = true;
        self.i64_remu = true;
        self.i64_and = true;
        self.i64_or = true;
        self.i64_xor = true;
        self.i64_shl = true;
        self.i64_shrs = true;
        self.i64_shru = true;
        self.i64_rotl = true;
        self.i64_rotr = true;
        //FL
        //F32
        self.f32_abs = true;
        self.f32_neg = true;
        self.f32_ceil = true;
        self.f32_floor = true;
        self.f32_trunc = true;
        self.f32_nearest = true;
        self.f32_sqrt = true;
        self.f32_add = true;
        self.f32_sub = true;
        self.f32_mul = true;
        self.f32_div = true;
        self.f32_min = true;
        self.f32_max = true;
        self.f32_copysign = true;
        //F64
        self.f64_abs = true;
        self.f64_neg = true;
        self.f64_ceil = true;
        self.f64_floor = true;
        self.f64_trunc = true;
        self.f64_nearest = true;
        self.f64_sqrt = true;
        self.f64_add = true;
        self.f64_sub = true;
        self.f64_mul = true;
        self.f64_div = true;
        self.f64_min = true;
        self.f64_max = true;
        self.f64_copysign = true;
        //tools
        self.i32_wrap_i64 = true;
        self.i32_trunc_f32s = true;
        self.i32_trunc_f32u = true;
        self.i32_trunc_f64s = true;
        self.i32_trunc_f64u = true;
        self.i64_extend_i32s = true;
        self.i64_extend_i32u = true;
        self.i64_trunc_f32s = true;
        self.i64_trunc_f32u = true;
        self.i64_trunc_f64s = true;
        self.i64_trunc_f64u = true;
        self.f32_convert_i32s = true;
        self.f32_convert_i32u = true;
        self.f32_convert_i64s = true;
        self.f32_convert_i64u = true;
        self.f32_demote_f64 = true;
        self.f64_convert_i32s = true;
        self.f64_converti_32u = true;
        self.f64_convert_i64s = true;
        self.f64_convert_i64u = true;
        self.f64_promote_f32 = true;
        self.i32_reinterpret_f32 = true;
        self.i64_reinterpret_f64 = true;
        self.f32_reinterpret_i32 = true;
        self.f64_reinterpret_i64 = true;
    }
    fn all_false(&mut self)
    {
        //I32
        self.i32_eqz = false;
        self.i32_eq = false;
        self.i32_ne = false;
        //flow
        self.unreachable = false;
        self.nop = false;
        self.block = false;
        self.pool = false;
        self.fi = false;
        self.esle = false;
        self.end = false;
        self.br = false;
        self.br_if = false;
        self.br_table = false;
        self.nruter = false;
        self.call = false;
        self.call_indirect = false;
        //Args
        self.drop = false;
        self.select = false;
        //Vars
        self.local_get = false;
        self.local_set = false;
        self.local_tee = false;
        self.global_get = false;
        self.global_set = false;

        //Mem
        //LD
        self.i32_load = false;
        self.i64_load = false;
        self.f32_load = false;
        self.f64_load = false;
        //I32
        self.i32_load_8s = false;
        self.i32_load_8u = false; 
        self.i32_load_16s = false;
        self.i32_load_16u = false;
        //I64
        self.i64_load_8s = false;
        self.i64_load_8u = false;
        self.i64_load_16s = false;
        self.i64_load_16u = false;
        self.i64_load_32s = false;
        self.i64_load_32u = false;
        //STR
        self.i32_store = false;
        self.i64_store = false;
        self.f32_store = false;
        self.f64_store = false;
        self.i32_store_8 = false;
        self.i32_store_16 = false;
        self.i64_store_8 = false;
        self.i64_store_16 = false;
        self.i64_store_32 = false;
        self.memory_size = false;
        self.memory_grow = false;
        //Cons
        self.i32_const = false;
        self.i64_const = false;
        self.f32_const = false;
        self.f64_const = false;
        //Comps    
        self.i32_lts = false;
        self.i32_ltu = false;
        self.i32_gts = false;
        self.i32_gtu = false;
        self.i32_les = false;
        self.i32_leu = false;
        self.i32_ges = false;
        self.i32_geu = false;
        //I64
        self.i64_eqz = false;
        self.i64_eq = false;
        self.i64_ne = false;
        self.i64_lts = false;
        self.i64_ltu = false;
        self.i64_gts = false;
        self.i64_gtu = false;
        self.i64_les = false;
        self.i64_leu = false;
        self.i64_ges = false;
        self.i64_geu = false;
        //F32
        self.f32_wq = false;
        self.f32_ne = false;
        self.f32_lt = false;
        self.f32_gt = false;
        self.f32_le = false;
        self.f32_ge = false;
        //F64
        self.f64_eq = false;
        self.f64_ne = false;
        self.f64_lt = false;
        self.f64_gt = false;
        self.f64_le = false;
        self.f64_ge = false;
        //Calcs
        //I32
        self.i32_clz = false;
        self.i32_ctz = false;
        self.i32_popcnt = false;
        self.i32_add = false;
        self.i32_sub = false;
        self.i32_mul = false;
        self.i32_divs = false;
        self.i32_divu = false;
        self.i32_rems = false;
        self.i32_remu = false;
        self.i32_and = false;
        self.i32_or = false;
        self.i32_xor = false;
        self.i32_shl = false;
        self.i32_shrs = false;
        self.i32_shru = false;
        self.i32_rotl = false;
        self.i32_rotr = false;
        //I64
        self.i64_clz = false;
        self.i64_ctz = false;
        self.i64_popcnt = false;
        self.i64_add = false;
        self.i64_sub = false;
        self.i64_mul = false;
        self.i64_divs = false;
        self.i64_divu = false;
        self.i64_rems = false;
        self.i64_remu = false;
        self.i64_and = false;
        self.i64_or = false;
        self.i64_xor = false;
        self.i64_shl = false;
        self.i64_shrs = false;
        self.i64_shru = false;
        self.i64_rotl = false;
        self.i64_rotr = false;
        //FL
        //F32
        self.f32_abs = false;
        self.f32_neg = false;
        self.f32_ceil = false;
        self.f32_floor = false;
        self.f32_trunc = false;
        self.f32_nearest = false;
        self.f32_sqrt = false;
        self.f32_add = false;
        self.f32_sub = false;
        self.f32_mul = false;
        self.f32_div = false;
        self.f32_min = false;
        self.f32_max = false;
        self.f32_copysign = false;
        //F64
        self.f64_abs = false;
        self.f64_neg = false;
        self.f64_ceil = false;
        self.f64_floor = false;
        self.f64_trunc = false;
        self.f64_nearest = false;
        self.f64_sqrt = false;
        self.f64_add = false;
        self.f64_sub = false;
        self.f64_mul = false;
        self.f64_div = false;
        self.f64_min = false;
        self.f64_max = false;
        self.f64_copysign = false;
        //tools
        self.i32_wrap_i64 = false;
        self.i32_trunc_f32s = false;
        self.i32_trunc_f32u = false;
        self.i32_trunc_f64s = false;
        self.i32_trunc_f64u = false;
        self.i64_extend_i32s = false;
        self.i64_extend_i32u = false;
        self.i64_trunc_f32s = false;
        self.i64_trunc_f32u = false;
        self.i64_trunc_f64s = false;
        self.i64_trunc_f64u = false;
        self.f32_convert_i32s = false;
        self.f32_convert_i32u = false;
        self.f32_convert_i64s = false;
        self.f32_convert_i64u = false;
        self.f32_demote_f64 = false;
        self.f64_convert_i32s = false;
        self.f64_converti_32u = false;
        self.f64_convert_i64s = false;
        self.f64_convert_i64u = false;
        self.f64_promote_f32 = false;
        self.i32_reinterpret_f32 = false;
        self.i64_reinterpret_f64 = false;
        self.f32_reinterpret_i32 = false;
        self.f64_reinterpret_i64 = false;
    }
    pub fn new() -> PFlags
    {
        PFlags {i32_eqz: false, i32_eq: false, i32_ne: false, unreachable: false, nop: false, 
            block: false, pool: false, fi: false, esle: false, end: false, br: false, br_if: false,
            br_table: false, nruter: false, call: false, call_indirect: false, drop: false, 
            select: false, local_get: false, local_set: false, local_tee: false, global_get: false, 
            global_set: false, i32_load: false, i64_load: false, f32_load: false, f64_load: false, 
            i32_load_8s: false, i32_load_8u: false, i32_load_16s: false, i32_load_16u: false, i64_load_8s: false, 
            i64_load_8u: false, i64_load_16s: false, i64_load_16u: false, i64_load_32s: false, i64_load_32u: false, 
            i32_store: false, i64_store: false, f32_store: false, f64_store: false, i32_store_8: false, i32_store_16: false, 
            i64_store_8: false, i64_store_16: false, i64_store_32: false, memory_size: false, memory_grow: false, i32_const: false, 
            i64_const: false, f32_const: false, f64_const: false, i32_lts: false, i32_ltu: false, i32_gts: false, i32_gtu: false, 
            i32_les: false, i32_leu: false, i32_ges: false, i32_geu: false, i64_eqz: false, i64_eq: false, i64_ne: false, i64_lts: false, 
            i64_ltu: false, i64_gts: false, i64_gtu: false, i64_les: false, i64_leu: false, i64_ges: false, i64_geu: false, f32_wq: false, 
            f32_ne: false, f32_lt: false, f32_gt: false, f32_le: false, f32_ge: false, f64_eq: false, f64_ne: false, f64_lt: false, 
            f64_gt: false, f64_le: false, f64_ge: false, i32_clz: false, i32_ctz: false, i32_popcnt: false, i32_add: false, i32_sub: false, 
            i32_mul: false, i32_divs: false, i32_divu: false, i32_rems: false, i32_remu: false, i32_and: false, i32_or: false, i32_xor: false, 
            i32_shl: false, i32_shrs: false, i32_shru: false, i32_rotl: false, i32_rotr: false, i64_clz: false, i64_ctz: false, i64_popcnt: false, 
            i64_add: false, i64_sub: false, i64_mul: false, i64_divs: false, i64_divu: false, i64_rems: false, i64_remu: false, i64_and: false, 
            i64_or: false, i64_xor: false, i64_shl: false, i64_shrs: false, i64_shru: false, i64_rotl: false, i64_rotr: false, f32_abs: false, 
            f32_neg: false, f32_ceil: false, f32_floor: false, f32_trunc: false, f32_nearest: false, f32_sqrt: false, f32_add: false, 
            f32_sub: false, f32_mul: false, f32_div: false, f32_min: false, f32_max: false, f32_copysign: false, f64_abs: false, f64_neg: false, 
            f64_ceil: false, f64_floor: false, f64_trunc: false, f64_nearest: false, f64_sqrt: false, f64_add: false, f64_sub: false, 
            f64_mul: false, f64_div: false, f64_min: false, f64_max: false, f64_copysign: false, i32_wrap_i64: false, i32_trunc_f32s: false, 
            i32_trunc_f32u: false, i32_trunc_f64s: false, i32_trunc_f64u: false, i64_extend_i32s: false, i64_extend_i32u: false, 
            i64_trunc_f32s: false, i64_trunc_f32u: false, i64_trunc_f64s: false, i64_trunc_f64u: false, f32_convert_i32s: false, 
            f32_convert_i32u: false, f32_convert_i64s: false, f32_convert_i64u: false, f32_demote_f64: false, f64_convert_i32s: false, 
            f64_converti_32u: false, f64_convert_i64s: false, f64_convert_i64u: false, f64_promote_f32: false, i32_reinterpret_f32: false, 
            i64_reinterpret_f64: false, f32_reinterpret_i32: false, f64_reinterpret_i64: false}
    }
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
    pub fpflags: PFlags,
    pub spflags: PFlags,
    #[serde(skip)]
    pub terminal: Option<Arc<Mutex<ChildStdin>>>,
    pub paused: bool,
    pub incount: usize,
    pub ended: bool,
    pub priority: usize,
    pub  limflag: bool,
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
            assert!(off + elm.fvec.len() <= functab.len());
            for byts in &elm.fvec
            {
                functab[off] = Some(*byts);
                off +=1;
            }
        }
        Runtime{fpflags: PFlags::new(), spflags: PFlags::new(), terminal: None, paused: false, incount: 0, ended: false, priority: 1, flog: false, clog: false, limflag: false, limit: 0, module, functab, mem: memvec, memmin, memmax, call_stack: Vec::new(), value_stack: Vec::new(), flow_stack: Vec::new(), globs,}
    } 
    pub fn pop_run(&mut self)
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
            if self.terminal.is_none() && let Ok(mut term) = Command::new("cmd").args(["/C", "start", "cmd", "/k", "more"]).stdin(Stdio::piped()).spawn()
            {
                    self.terminal = Some(Arc::new(Mutex::new(term.stdin.take().expect("Terminal Child Process error"))));
            }
            else{
                println!("Terminal could not be created Runtime: {}", self.module.name);
                return;
            }
            if let Some(tpipe) = &self.terminal 
                && let Ok(mut pip) = tpipe.lock() 
                    && let Err(_err) = writeln!(pip, "{logg}\n")
            {
                println!("Could not write to terminal Runtime: {}", self.module.name);
            }
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
        let code = call.code[call.loc].clone();
        call.loc += 1;
        match code
        {
            //flow
            Code::Unreachable => panic!("wasm module reached unreachable instruction"),
            Code::Nop => (), //instruction is a placeholder in wasm
            Code::Block(typ) => self.flow_stack.push(FlowCode{flow_type: FlowType::Block, break_tar: call.code.len() - 1, size: self.value_stack.len(), ret_typ: typ}),
            Code::Loop(typ) => self.flow_stack.push(FlowCode{ flow_type: FlowType::Loop, break_tar: call.code.len(), size: self.value_stack.len(), ret_typ: typ,}),    
            //Code::If(typ) => self.flow_stack.push(FlowCode{flow_type: FlowType::If, break_tar: , size: false, ret_typ: false }),
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
                lstring = format!("{}. Call {}", self.incount, ind);
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
            },
            /*Code::CallIndirect(ind) => 
            {

            },*/
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
                let val2 = self.value_stack.pop().expect("Stack Sel Fail");
                let val1 = self.value_stack.pop().expect("Stack Sel Fail");

                if sel != 0 {self.value_stack.push(val1);}
                else{self.value_stack.push(val2);}
            },
            //Vars
            Code::LocalGet(loc) => {
                let val = call.vars.get(loc as usize).unwrap().clone();
                lstring = format!("{}. Local Get({}): {:?}", self.incount, loc, val);
                self.value_stack.push(val);
                //log::info!("Local Get: Index: {}, Value: {}", loc, val);
            },
            Code::LocalSet(loc) => {
                let to_stack = self.value_stack.pop().unwrap();
                lstring = format!("{}. Local Set({}) {:?}", self.incount, loc, to_stack);
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
                lstring = format!("{}. LocalTee({}) {:?}", self.incount, loc, to_loc);
                call.vars[ind] = to_loc;

            },
            Code::GlobalGet(loc) =>
            {
                let loc = loc as usize;
                assert!(loc <= self.globs.len());
                let to_stack = self.globs[loc].typ.clone();
                lstring = format!("{}. Global Get({}) {:?}", self.incount, loc, to_stack);
                self.value_stack.push(to_stack);
                //log::info!("Global Get: Index: {}, Value: {}", loc, to_stack);
            },
            Code::GlobalSet(loc) =>
            {
                let to_glob = self.value_stack.pop().expect("Stack empty globset");
                assert!(self.globs[loc as usize].ismut);
                lstring = format!("{}. Global Set({}) {:?}", self.incount, loc, to_glob);
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
                lstring = format!("{}. I32Store({}) {:?}", self.incount, off, bytes);
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
                lstring = format!("{}. I64Store({}) {:?}", self.incount, off, bytes);
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
                lstring = format!("{}. F32Store({}) {:?}", self.incount, off, bytes);
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
                lstring = format!("{}. F64Store({}) {:?}", self.incount, off, bytes);
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
                lstring = format!("{}. I32Store8({}) {}", self.incount, off, var);
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
                lstring = format!("{}. I32Store16({}) {}", self.incount, off, var);
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
                lstring = format!("{}. I64Store8({}) {}", self.incount, off, var);
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
                lstring = format!("{}. I64Store16({}) {}", self.incount, off, var);
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
                lstring = format!("{}. I64Store32({}) {}", self.incount, off, var);
                self.mem[uloc..uloc + 4].copy_from_slice(&var.to_le_bytes()); 
            },
            Code::MemorySize => 
            {
                let memlen = self.mem.len();
                lstring = format!("{}. MemorySize {} ", self.incount, memlen);
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
                lstring = format!("{}. MemoryGrow New: {} Old: {}", self.incount, newmem, curmem);
                self.value_stack.push(StackTypes::I32(curmem));

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
                let i_val = match self.value_stack.pop()
                {
                    Some(StackTypes::I32(val)) => val,
                    _ => panic!("Invalid type stack error"),
                };
                lstring = format!("{}. I32Eqz {}", self.incount, i_val);
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
                lstring = format!("{}. I32Eq Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. I32Ne Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. I32LtS Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. I32LtU Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. I32GtS Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. I32GtU Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. I32LeS Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. I32LeU Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. I32GeS Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. I32GeU Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. I64Eqz {}", self.incount, val);
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
                lstring = format!("{}. I64Eq Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. F32Eq Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. F32Ne Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. F32Lt Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. F32Gt Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. F32Le Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. I32Clz {}", self.incount, val);
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
                lstring = format!("{}. I32Ctz {}", self.incount, val);
                self.value_stack.push(StackTypes::I32(trailing_zeros as i32));
            },  
//                Code::I32Popcnt => (),
            Code::I32Popcnt => {
                let val = match self.value_stack.pop()
                {
                    Some(StackTypes::I32(v)) => v,
                    _ => panic!("Invalid type stack error"),
                };
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
                lstring = format!("{}. I32Sub Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. I32Mul Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. I32DivS Val1: {}/ Val2: {}", self.incount, val1, val2);
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
            lstring = format!("{}. I32DivU Val1: {}/ Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. I32RemS Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. I32RemU Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. I64Sub Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. I64Mul Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. I64DivS Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. I64DivU Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. I64RemS Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. I64RemU Val1: {} Val2: {}", self.incount, a, b);
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
                lstring = format!("{}. I64ShrS Shift: {} Val: {}", self.incount, shift, value);
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
                lstring = format!("{}. F64Ge Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. F64eq Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. F64Ne Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. F64Lt Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. F64Gt Val1: {} Val2: {}", self.incount, val1, val2);
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
                lstring = format!("{}. F64Le Val1: {} Val2: {}", self.incount, val1, val2);
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
            _ => panic!("Unsupported Type"),
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