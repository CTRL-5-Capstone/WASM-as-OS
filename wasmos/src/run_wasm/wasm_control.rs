


/// Execute a WASM file by path, returning a rich ExecutionResult.
/// Parses the WASM, initializes a Runtime, runs to completion, and collects metrics.
pub fn execute_wasm_file(path_str: &str) -> Result<super::execution_result::ExecutionResult, String> {
    let path = std::path::Path::new(path_str);
    if !path.exists() {
        return Err(format!("WASM file not found: {}", path_str));
    }

    // Pre-check: empty files are not valid WASM; return a controlled failure
    // rather than letting the engine panic inside catch_unwind.
    let file_size = std::fs::metadata(path)
        .map(|m| m.len())
        .unwrap_or(0);
    if file_size == 0 {
        return Ok(super::execution_result::ExecutionResult::failure(
            "WASM file is empty (0 bytes)".to_string(),
            0, 0, 0, 0,
            vec!["[ERROR] Empty WASM file — nothing to execute".to_string()],
        ));
    }

    let file_name = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let path_owned = path.to_path_buf();
    let start = std::time::Instant::now();

    // Catch panics from the WASM engine — the custom engine uses panic!() for
    // unsupported opcodes, invalid modules, out-of-bounds access, etc.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut runtime = super::wasm_engine::wasm_engine(file_name, &path_owned);

        // wasm_engine() already calls pop_run(), so don't call it again

        // Run to completion, with a safety limit to prevent infinite game loops
        // (e.g. snake.wasm has an infinite update loop that never sets `ended`).
        // 10 million instructions is generous for init/short programs but still bounded.
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

            Ok(super::execution_result::ExecutionResult::success(
                runtime.instruction_count,
                runtime.syscall_count,
                memory_used,
                duration_us,
                runtime.stdout_log,
                return_value,
            ))
        }
        Err(panic_info) => {
            // Extract panic message
            let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "WASM engine encountered an unsupported feature or invalid module".to_string()
            };

            Ok(super::execution_result::ExecutionResult::failure(
                format!("WASM execution panic: {}", panic_msg),
                0,
                0,
                0,
                duration_us,
                vec![format!("[ERROR] Engine panic: {}", panic_msg)],
            ))
        }
    }
}

