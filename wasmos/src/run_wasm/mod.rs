pub mod wasm_control;
pub mod wasm_engine;
pub mod wasm_module;
pub mod build_runtime;
pub mod trap;
pub mod execution_result;
pub mod advanced_executor;
pub mod import_call_manager;
pub mod execution_framework;
pub mod wasm_imports;
pub mod interpreter;
pub mod syscall_policy;

#[cfg(test)]
mod tests;

pub use execution_result::ExecutionResult;
pub use wasm_control::execute_wasm_file;
pub use execution_framework::{ExecutionDispatcher, ExecutionConfig};
pub use syscall_policy::{SyscallPolicy, SyscallViolation, PolicyAction, PolicyPreset, PolicyRequest};
#[allow(unused_imports)]
pub use trap::WasmTrap;

