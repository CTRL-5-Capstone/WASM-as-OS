#[derive(Clone)]
pub struct Module
{
    pub name: String,
    pub imps: Vec<Import>,
    pub typs: Vec<Types>,
    pub fnid: Vec<u32>,
    pub tabs: Vec<Tab>,
    pub memy: Vec<MemoIn>,
    pub glob: Vec<Global>,
    pub exps: Vec<Export>,
    pub strt: Option<u32>,
    pub elms: Vec<Element>,
    pub fcce: Vec<Function>,
    pub mmsg: Vec<MemSeg>,
    pub imports: u32,
    //pub memcount: u32
}
impl Module
{
    pub fn new() -> Self
    {
        Self 
        {
            name: String::new(),
            imps: Vec::new(),
            typs: Vec::new(),
            fnid: Vec::new(),
            tabs: Vec::new(),
            memy: Vec::new(),
            glob: Vec::new(),
            exps: Vec::new(),
            strt: None,
            elms: Vec::new(),
            fcce: Vec::new(),
            mmsg: Vec::new(),
            imports: 0,
            //memcount: 0,

        }
    }
}
#[derive(Clone)]
pub enum TypeBytes
{
    I32,
    I64,
    F32,
    F64,
}

pub fn decode_byte(byte: u8) -> Option<TypeBytes> //Decode byte to type
{
    match byte
    {
        0x7C => Some(TypeBytes::F64),
        0x7D => Some(TypeBytes::F32),
        0x7E => Some(TypeBytes::I64),
        0x7F => Some(TypeBytes::I32),
        _ => None,
    }
}
#[derive(Clone)]
pub struct Import
{
    pub modname: String,
    pub impname: String,
    pub imptyp: ExpTyp,
    pub index: Option<u32>,
    pub tab: Option<Tab>,
    pub mem: Option<MemoIn>,
    pub glob: Option<ShortGlobal>,
}
#[derive(Clone)]
pub struct MemoIn{
    pub flag: u8,
    pub memmin: u32,
    pub memmax: Option<u32>,
}
#[derive(Clone)]
pub struct ShortGlobal
{
    pub typ: TypeBytes,
    pub is_mut: bool,
}
#[derive(Clone)]
pub struct Types
{
    pub args: Vec<Option<TypeBytes>>,
    pub turns: Vec<Option<TypeBytes>>,
}
#[derive(Clone)]
pub struct Export
{
    pub name: String,
    pub loc: u32,
    pub typ: ExpTyp,
}
#[derive(Clone)]
pub enum ExpTyp
{
    Memory,
    Func,
    Table,
    Global,

}
#[derive(Clone)]
pub struct Global
{
    pub typ: TypeBytes,
    pub ismut: bool,
    pub code: Code,
}
#[derive(Clone)]
pub struct Element
{
    pub tabid: u32,
    pub elmtyp: Option<u8>, 
    pub elmoff: Code,
    pub fvec: Vec<u32>,
}
/*#[derive(Clone)] for wasm 1 plus
pub enum MemTyp //Could probably just store the flag instead looking back while working on element section
{
    Waiting,
    Immediate,
}*/
#[derive(Clone)]
pub struct MemSeg
{
    //pub memtyp: MemTyp,
    //pub code:  Vec<Code>,
    pub code: Code,
    pub dvec: Vec<u8>,
}
#[derive(Clone)]
pub struct Function
{
    pub vars: Vec<(u32, Option<TypeBytes>)>,
    pub code: Vec<Code>
}
#[derive(Clone)]
pub struct Tab
{
    pub typ: u8,
    pub flag: u8,
    pub tabmin: u32,
    pub tabmax: Option<u32>,
    //pub tabmin64: Option<i64>,
    //pub tabmax64: Option<i64>
}
#[derive(Clone)]
pub enum Code
{
    //The grandest of enumerations!
    //Codes for the interpreter when running the module

    //I32
    I32Eqz,
    I32Eq,
    I32Ne,
   //flow
    Unreachable,
    Nop,
    Block(Option<TypeBytes>),
    Loop(Option<TypeBytes>),
    If(Option<TypeBytes>),
    Else,
    End,
    Br(u32),
    BrIf(u32),
    BrTable
    {
        def: u32,
        locs: Vec<u32>,
    },
    Return,
    Call(u32),
    CallIndirect(u32),
    //Args
    Drop,
    Select,
    //Vars
    LocalGet(u32),
    LocalSet(u32),
    LocalTee(u32),
    GlobalGet(u32),
    GlobalSet(u32),

    //Mem
    //LD
    I32Load(u32),
    I64Load(u32),
    F32Load(u32),
    F64Load(u32),
    //I32
    I32Load8S(u32),
    I32Load8U(u32),
    I32Load16S(u32),
    I32Load16U(u32),
    //I64
    I64Load8S(u32),
    I64Load8U(u32),
    I64Load16S(u32),
    I64Load16U(u32),
    I64Load32S(u32),
    I64Load32U(u32),
    //STR
    I32Store(u32),
    I64Store(u32),
    F32Store(u32),
    F64Store(u32),
    I32Store8(u32),
    I32Store16(u32),
    I64Store8(u32),
    I64Store16(u32),
    I64Store32(u32),
    MemorySize,
    MemoryGrow,
    //Cons
    I32Const(i32),
    I64Const(i64),
    F32Const(f32),
    F64Const(f64),
    //Comps    
    I32LtS,
    I32LtU,
    I32GtS,
    I32GtU,
    I32LeS,
    I32LeU,
    I32GeS,
    I32GeU,
    //I64
    I64Eqz,
    I64Eq,
    I64Ne,
    I64LtS,
    I64LtU,
    I64GtS,
    I64GtU,
    I64LeS,
    I64LeU,
    I64GeS,
    I64GeU,
    //F32
    F32Eq,
    F32Ne,
    F32Lt,
    F32Gt,
    F32Le,
    F32Ge,
    //F64
    F64Eq,
    F64Ne,
    F64Lt,
    F64Gt,
    F64Le,
    F64Ge,
    //Calcs
    //I32
    I32Clz,
    I32Ctz,
    I32Popcnt,
    I32Add,
    I32Sub,
    I32Mul,
    I32DivS,
    I32DivU,
    I32RemS,
    I32RemU,
    I32And,
    I32Or,
    I32Xor,
    I32Shl,
    I32ShrS,
    I32ShrU,
    I32Rotl,
    I32Rotr,
    //I64
    I64Clz,
    I64Ctz,
    I64Popcnt,
    I64Add,
    I64Sub,
    I64Mul,
    I64DivS,
    I64DivU,
    I64RemS,
    I64RemU,
    I64And,
    I64Or,
    I64Xor,
    I64Shl,
    I64ShrS,
    I64ShrU,
    I64Rotl,
    I64Rotr,
    //FL
    //F32
    F32Abs,
    F32Neg,
    F32Ceil,
    F32Floor,
    F32Trunc,
    F32Nearest,
    F32Sqrt,
    F32Add,
    F32Sub,
    F32Mul,
    F32Div,
    F32Min,
    F32Max,
    F32Copysign,
    //F64
    F64Abs,
    F64Neg,
    F64Ceil,
    F64Floor,
    F64Trunc,
    F64Nearest,
    F64Sqrt,
    F64Add,
    F64Sub,
    F64Mul,
    F64Div,
    F64Min,
    F64Max,
    F64Copysign,
    //tools
    I32WrapI64,
    I32TruncF32S,
    I32TruncF32U,
    I32TruncF64S,
    I32TruncF64U,
    I64ExtendI32S,
    I64ExtendI32U,
    I64TruncF32S,
    I64TruncF32U,
    I64TruncF64S,
    I64TruncF64U,
    F32ConvertI32S,
    F32ConvertI32U,
    F32ConvertI64S,
    F32ConvertI64U,
    F32DemoteF64,
    F64ConvertI32S,
    F64ConvertI32U,
    F64ConvertI64S,
    F64ConvertI64U,
    F64PromoteF32,
    I32ReinterpretF32,
    I64ReinterpretF64,
    F32ReinterpretI32,
    F64ReinterpretI64,
}

