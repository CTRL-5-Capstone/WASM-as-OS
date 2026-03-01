use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum WasmTrap {
    #[error("Division by zero")]
    DivisionByZero,
    
    #[error("Integer overflow")]
    IntegerOverflow,
    
    #[error("Out of bounds memory access at address {address:#x}")]
    OutOfBoundsMemory { address: usize },
    
    #[error("Out of bounds table access at index {index}")]
    OutOfBoundsTable { index: usize },
    
    #[error("Undefined element at index {index}")]
    UndefinedElement { index: usize },
    
    #[error("Unreachable instruction executed")]
    Unreachable,
    
    #[error("Call stack exhausted (max depth: {max_depth})")]
    StackExhausted { max_depth: usize },
    
    #[error("Out of gas (consumed: {consumed}, limit: {limit})")]
    OutOfGas { consumed: u64, limit: u64 },
    
    #[error("Execution timeout after {seconds} seconds")]
    Timeout { seconds: u64 },
    
    #[error("Invalid function index: {index}")]
    InvalidFunction { index: u32 },
    
    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },
    
    #[error("Invalid conversion")]
    InvalidConversion,
}

pub type TrapResult<T> = Result<T, WasmTrap>;

impl WasmTrap {
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            WasmTrap::OutOfGas { .. } | WasmTrap::Timeout { .. }
        )
    }
}
