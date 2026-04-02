/// WASM execution traps.
///
/// A trap is an abnormal termination of a WASM module — such as reaching an
/// `unreachable` instruction, a stack overflow, an out-of-bounds memory access,
/// or an integer divide-by-zero.  The engine converts these into `WasmTrap`
/// values rather than panicking so callers can handle them gracefully.
use std::fmt;

/// Reason a WASM execution terminated with a trap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasmTrap {
    /// The `unreachable` instruction was executed (e.g. a compiled Rust panic).
    Unreachable,
    /// A memory access was outside the bounds of the linear memory.
    MemoryOutOfBounds { offset: u64, size: u64, mem_size: u64 },
    /// The call stack exceeded the maximum supported depth.
    StackOverflow,
    /// Integer divide-by-zero or integer overflow in a trapping operation.
    IntegerOverflow,
    /// `call_indirect` type mismatch — actual function type differs from expected.
    IndirectCallTypeMismatch,
    /// A table index was out of bounds.
    TableOutOfBounds { index: u64 },
    /// Any other engine-detected trap with a descriptive message.
    Other(String),
}

impl fmt::Display for WasmTrap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WasmTrap::Unreachable => write!(f, "unreachable instruction executed"),
            WasmTrap::MemoryOutOfBounds { offset, size, mem_size } => {
                write!(f, "memory access out of bounds: offset={offset} size={size} mem={mem_size}")
            }
            WasmTrap::StackOverflow => write!(f, "call stack exhausted"),
            WasmTrap::IntegerOverflow => write!(f, "integer overflow / divide by zero"),
            WasmTrap::IndirectCallTypeMismatch => {
                write!(f, "indirect call type mismatch")
            }
            WasmTrap::TableOutOfBounds { index } => {
                write!(f, "table index out of bounds: {index}")
            }
            WasmTrap::Other(msg) => write!(f, "trap: {msg}"),
        }
    }
}

impl std::error::Error for WasmTrap {}

/// Convert a trap into a one-line error string suitable for `ExecutionResult::failure`.
impl From<WasmTrap> for String {
    fn from(t: WasmTrap) -> Self {
        t.to_string()
    }
}
