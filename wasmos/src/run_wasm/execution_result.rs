use serde::{Deserialize, Serialize};
use crate::run_wasm::syscall_policy::SyscallViolation;

/// Rich execution result returned by the WASM engine after running a module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Whether execution completed successfully
    pub success: bool,
    /// Error message if execution failed
    pub error: Option<String>,
    /// Total instructions executed
    pub instructions_executed: u64,
    /// Total ABI syscalls invoked (allowed calls only)
    pub syscalls_executed: u64,
    /// Number of import calls blocked by the syscall policy
    pub blocked_syscall_count: u64,
    /// Detailed violation records — empty when policy is permissive
    pub syscall_violations: Vec<SyscallViolation>,
    /// Peak memory usage in bytes
    pub memory_used_bytes: u64,
    /// Execution duration in microseconds
    pub duration_us: u64,
    /// Output from host_log syscalls
    pub stdout_log: Vec<String>,
    /// String representation of the return value (if any)
    pub return_value: Option<String>,
    /// Human-readable label of the policy that was applied
    pub policy_label: String,
}

impl ExecutionResult {
    pub fn success(
        instructions: u64,
        syscalls: u64,
        blocked: u64,
        violations: Vec<SyscallViolation>,
        memory_bytes: u64,
        duration_us: u64,
        stdout_log: Vec<String>,
        return_value: Option<String>,
        policy_label: String,
    ) -> Self {
        Self {
            success: true,
            error: None,
            instructions_executed: instructions,
            syscalls_executed: syscalls,
            blocked_syscall_count: blocked,
            syscall_violations: violations,
            memory_used_bytes: memory_bytes,
            duration_us,
            stdout_log,
            return_value,
            policy_label,
        }
    }

    pub fn failure(
        error: String,
        instructions: u64,
        syscalls: u64,
        blocked: u64,
        violations: Vec<SyscallViolation>,
        memory_bytes: u64,
        duration_us: u64,
        stdout_log: Vec<String>,
        policy_label: String,
    ) -> Self {
        Self {
            success: false,
            error: Some(error),
            instructions_executed: instructions,
            syscalls_executed: syscalls,
            blocked_syscall_count: blocked,
            syscall_violations: violations,
            memory_used_bytes: memory_bytes,
            duration_us,
            stdout_log,
            return_value: None,
            policy_label,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_wasm::syscall_policy::SyscallViolation;

    // Helper: build an empty violation list for tests that don't care about violations.
    fn no_violations() -> Vec<SyscallViolation> { vec![] }

    #[test]
    fn test_success_fields() {
        // success(instructions, syscalls, blocked, violations, memory_bytes,
        //         duration_us, stdout_log, return_value, policy_label)
        let res = ExecutionResult::success(
            100, 5, 0, no_violations(),
            65536, 1234,
            vec!["hello".to_string()],
            Some("42".to_string()),
            "permissive".to_string(),
        );
        assert!(res.success);
        assert!(res.error.is_none());
        assert_eq!(res.instructions_executed, 100);
        assert_eq!(res.syscalls_executed, 5);
        assert_eq!(res.memory_used_bytes, 65536);
        assert_eq!(res.duration_us, 1234);
        assert_eq!(res.stdout_log, vec!["hello"]);
        assert_eq!(res.return_value, Some("42".to_string()));
        assert_eq!(res.policy_label, "permissive");
    }

    #[test]
    fn test_failure_fields() {
        // failure(error, instructions, syscalls, blocked, violations,
        //         memory_bytes, duration_us, stdout_log, policy_label)
        let res = ExecutionResult::failure(
            "trap: unreachable".to_string(),
            50, 2, 0, no_violations(),
            32768, 500,
            vec!["log line".to_string()],
            "permissive".to_string(),
        );
        assert!(!res.success);
        assert_eq!(res.error, Some("trap: unreachable".to_string()));
        assert_eq!(res.instructions_executed, 50);
        assert_eq!(res.syscalls_executed, 2);
        assert_eq!(res.memory_used_bytes, 32768);
        assert_eq!(res.duration_us, 500);
        assert!(res.return_value.is_none());
    }

    #[test]
    fn test_success_no_return_value() {
        let res = ExecutionResult::success(
            0, 0, 0, no_violations(), 0, 0, vec![], None, "permissive".to_string(),
        );
        assert!(res.success);
        assert!(res.return_value.is_none());
        assert!(res.stdout_log.is_empty());
    }

    #[test]
    fn test_success_serializes_json() {
        let res = ExecutionResult::success(
            10, 0, 0, no_violations(), 0, 100, vec![], None, "permissive".to_string(),
        );
        let json = serde_json::to_string(&res).expect("serialize failed");
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"instructions_executed\":10"));
        assert!(json.contains("\"duration_us\":100"));
    }

    #[test]
    fn test_failure_serializes_json() {
        let res = ExecutionResult::failure(
            "oops".into(), 0, 0, 0, no_violations(), 0, 0, vec![], "permissive".to_string(),
        );
        let json = serde_json::to_string(&res).expect("serialize failed");
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"error\":\"oops\""));
    }

    #[test]
    fn test_failure_preserves_stdout_log() {
        let logs = vec!["line1".to_string(), "line2".to_string()];
        let res = ExecutionResult::failure(
            "err".into(), 0, 0, 0, no_violations(), 0, 0, logs.clone(), "permissive".to_string(),
        );
        assert_eq!(res.stdout_log, logs);
    }

    #[test]
    fn test_violations_are_recorded_in_failure() {
        let v = SyscallViolation::new("dangerous_call", "env", 99, "blocked".to_string());
        let res = ExecutionResult::failure(
            "policy halt".into(), 10, 0, 1, vec![v],
            0, 50, vec![], "strict".to_string(),
        );
        assert_eq!(res.blocked_syscall_count, 1);
        assert_eq!(res.syscall_violations.len(), 1);
        assert_eq!(res.syscall_violations[0].name, "dangerous_call");
        assert_eq!(res.policy_label, "strict");
    }
}