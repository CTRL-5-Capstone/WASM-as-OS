use serde::Serialize;

/// Rich execution result returned by the WASM engine after running a module.
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionResult {
    /// Whether execution completed successfully
    pub success: bool,
    /// Error message if execution failed
    pub error: Option<String>,
    /// Total instructions executed
    pub instructions_executed: u64,
    /// Total ABI syscalls invoked
    pub syscalls_executed: u64,
    /// Peak memory usage in bytes
    pub memory_used_bytes: u64,
    /// Execution duration in microseconds
    pub duration_us: u64,
    /// Output from host_log syscalls
    pub stdout_log: Vec<String>,
    /// String representation of the return value (if any)
    pub return_value: Option<String>,
}

impl ExecutionResult {
    pub fn success(
        instructions: u64,
        syscalls: u64,
        memory_bytes: u64,
        duration_us: u64,
        stdout_log: Vec<String>,
        return_value: Option<String>,
    ) -> Self {
        Self {
            success: true,
            error: None,
            instructions_executed: instructions,
            syscalls_executed: syscalls,
            memory_used_bytes: memory_bytes,
            duration_us,
            stdout_log,
            return_value,
        }
    }

    pub fn failure(
        error: String,
        instructions: u64,
        syscalls: u64,
        memory_bytes: u64,
        duration_us: u64,
        stdout_log: Vec<String>,
    ) -> Self {
        Self {
            success: false,
            error: Some(error),
            instructions_executed: instructions,
            syscalls_executed: syscalls,
            memory_used_bytes: memory_bytes,
            duration_us,
            stdout_log,
            return_value: None,
        }
    }
}
