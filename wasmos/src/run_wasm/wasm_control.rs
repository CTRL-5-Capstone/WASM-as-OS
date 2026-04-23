use super::syscall_policy::{SyscallPolicy, SyscallViolation};

/// Execute a WASM file by path with an optional syscall policy.
/// Parses the WASM, initialises a Runtime with the given policy,
/// runs to completion, and collects rich metrics + violation data.
///
/// Passing `None` for `policy` defaults to the permissive (allow-all) policy
/// which preserves backward-compatible behaviour.
pub fn execute_wasm_file(
    path_str: &str,
    policy: Option<SyscallPolicy>,
) -> Result<super::execution_result::ExecutionResult, String> {
    let policy = policy.unwrap_or_else(SyscallPolicy::permissive);
    let policy_label = policy.label.clone();

    let path = std::path::Path::new(path_str);
    if !path.exists() {
        return Err(format!("WASM file not found: {}", path_str));
    }

    // Pre-check: empty files are not valid WASM.
    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if file_size == 0 {
        return Ok(super::execution_result::ExecutionResult::failure(
            "WASM file is empty (0 bytes)".to_string(),
            0, 0, 0, vec![],
            0, 0,
            vec!["[ERROR] Empty WASM file — nothing to execute".to_string()],
            policy_label,
        ));
    }

    let file_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let path_owned = path.to_path_buf();
    let start = std::time::Instant::now();

    // Capture the policy clone for the closure (catch_unwind needs 'static-ish data).
    let policy_for_runtime = policy.clone();

    // Catch panics from the WASM engine — the custom engine uses panic!() for
    // unsupported opcodes, invalid modules, out-of-bounds access, etc.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut runtime = super::wasm_engine::wasm_engine_with_policy(
            file_name,
            &path_owned,
            policy_for_runtime,
        );

        // wasm_engine_with_policy() already calls pop_run(), so don't call it again.

        // Safety limit to prevent infinite game loops.
        const MAX_INSTRUCTIONS: u64 = 10_000_000;
        while !runtime.ended && runtime.instruction_count < MAX_INSTRUCTIONS {
            runtime.run_prog();
        }

        runtime
    }));

    let duration_us = start.elapsed().as_micros() as u64;

    match result {
        Ok(runtime) => {
            let memory_used = runtime.mem.len() as u64;
            let return_value = if !runtime.value_stack.is_empty() {
                Some(format!("{:?}", runtime.value_stack.last().unwrap()))
            } else {
                None
            };
            let blocked = runtime.violations.len() as u64;
            let violations: Vec<SyscallViolation> = runtime.violations;

            // If there was a policy halt (violations present) but the runtime
            // didn't set success=false yet, we surface that through the error field.
            let was_blocked = blocked > 0;

            if was_blocked {
                Ok(super::execution_result::ExecutionResult::failure(
                    format!(
                        "Execution halted: {} syscall violation(s) detected",
                        blocked
                    ),
                    runtime.instruction_count,
                    runtime.syscall_count,
                    blocked,
                    violations,
                    memory_used,
                    duration_us,
                    runtime.stdout_log,
                    policy_label,
                ))
            } else {
                Ok(super::execution_result::ExecutionResult::success(
                    runtime.instruction_count,
                    runtime.syscall_count,
                    0,
                    vec![],
                    memory_used,
                    duration_us,
                    runtime.stdout_log,
                    return_value,
                    policy_label,
                ))
            }
        }
        Err(panic_info) => {
            let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "WASM engine encountered an unsupported feature or invalid module".to_string()
            };

            Ok(super::execution_result::ExecutionResult::failure(
                format!("WASM execution panic: {}", panic_msg),
                0, 0, 0, vec![],
                0,
                duration_us,
                vec![format!("[ERROR] Engine panic: {}", panic_msg)],
                policy_label,
            ))
        }
    }
}
