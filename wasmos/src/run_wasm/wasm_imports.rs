// wasm_imports.rs – Host import library for the WASM engine

use crate::run_wasm::execution_result::ExecutionResult;
use crate::run_wasm::wasm_module::Import;
use std::collections::HashMap;

/// Represents a host function callable from WASM.
/// The function receives a mutable reference to the runtime's memory and a mutable
/// reference to the execution result for logging / metric collection.
pub type HostFn = fn(&mut Vec<u8>, &mut ExecutionResult);

/// Central registry of host imports.
#[derive(Default)]
pub struct WasmImportLibrary {
    /// Mapping from import name to host function implementation.
    pub functions: HashMap<String, HostFn>,
}

#[allow(dead_code)]
impl WasmImportLibrary {
    /// Create a library with the default set of host functions.
    pub fn new() -> Self {
        let mut lib = WasmImportLibrary {
            functions: HashMap::new(),
        };
        // Basic logging function used by existing code.
        lib.register("host_log", Self::host_log);
        // Example sensor read – returns a dummy value.
        lib.register("read_sensor", Self::read_sensor);
        // Simple alert function.
        lib.register("send_alert", Self::send_alert);
        // Add more WASI or custom functions here as needed.
        lib
    }

    /// Register a host function.
    pub fn register(&mut self, name: &str, func: HostFn) {
        self.functions.insert(name.to_string(), func);
    }

    /// Dispatch an import call by name.
    pub fn dispatch(&self, name: &str, memory: &mut Vec<u8>, result: &mut ExecutionResult) {
        if let Some(f) = self.functions.get(name) {
            f(memory, result);
        } else {
            // Unknown import – log and push a zero return value if needed.
            println!("[WASM IMPORT] unknown import: {}", name);
            result.stdout_log.push(format!("[unknown import] {}", name));
        }
    }

    // ---------------------------------------------------------------------
    // Default host functions
    // ---------------------------------------------------------------------
    fn host_log(_memory: &mut Vec<u8>, result: &mut ExecutionResult) {
        // Expect two i32 arguments on the stack: ptr and len.
        // The caller (runtime) already popped them and placed them in the
        // result's value stack; we simply read from memory here.
        // This implementation is a placeholder – the real runtime will push
        // the arguments onto the stack before calling.
        // For now we just record that the host_log was invoked.
        result.stdout_log.push("[host_log called]".to_string());
        result.syscalls_executed += 1;
    }

    fn read_sensor(_memory: &mut Vec<u8>, result: &mut ExecutionResult) {
        // Return a dummy sensor reading.
        result.stdout_log.push("[read_sensor called]".to_string());
        // In a real implementation we would push the value onto the runtime
        // stack; the runtime will handle that after the dispatch.
        result.syscalls_executed += 1;
    }

    fn send_alert(_memory: &mut Vec<u8>, result: &mut ExecutionResult) {
        result.stdout_log.push("[send_alert called]".to_string());
        result.syscalls_executed += 1;
    }
}

/// Convert an Import descriptor into the host function name.
#[allow(dead_code)]
pub fn import_name(import: &Import) -> String {
    import.impname.clone()
}
