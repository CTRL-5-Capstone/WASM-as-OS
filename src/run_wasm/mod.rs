pub mod wasm_control;
pub mod wasm_engine;
pub mod wasm_module;
pub mod build_runtime;
pub mod trap;
pub mod execution_result;

pub use execution_result::ExecutionResult;
pub use wasm_control::execute_wasm_file;
