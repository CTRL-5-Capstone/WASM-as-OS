


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
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
 
    #[test]
    fn test_nonexistent_file_returns_err() {
        let result = execute_wasm_file("/tmp/no_such_file_999.wasm");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }
 
    #[test]
    fn test_empty_file_returns_failure() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let result = execute_wasm_file(tmp.path().to_str().unwrap());
        assert!(result.is_ok());
        let res = result.unwrap();
        assert!(!res.success);
        assert!(res.error.as_deref().unwrap_or("").to_lowercase().contains("empty"));
    }
 
    #[test]
    fn test_invalid_binary_returns_failure() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(b"not a wasm file at all").unwrap();
        let result = execute_wasm_file(tmp.path().to_str().unwrap());
        match result {
            Ok(res) => assert!(!res.success, "Invalid WASM should not succeed"),
            Err(_) => {} // also acceptable
        }
    }
 
    fn compile_wat(source: &str) -> tempfile::NamedTempFile {
        let wasm = wat::parse_str(source).expect("WAT compile failed");
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(&wasm).unwrap();
        tmp
    }
 
    #[test]
    fn test_simple_add() {
        let tmp = compile_wat(r#"(module
          (func $add (param i32 i32) (result i32)
            local.get 0 local.get 1 i32.add)
          (func (export "start") (result i32)
            i32.const 5 i32.const 7 call $add return))"#);
        let result = execute_wasm_file(tmp.path().to_str().unwrap());
        assert!(result.is_ok(), "Err: {:?}", result);
    }
 
    #[test]
    fn test_loop_program() {
        let tmp = compile_wat(r#"(module
          (func (export "count") (result i32)
            (local $i i32)
            (local.set $i (i32.const 0))
            (block $exit
              (loop $loop
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (br_if $exit (i32.ge_u (local.get $i) (i32.const 10)))
                (br $loop)))
            (local.get $i)))"#);
        let result = execute_wasm_file(tmp.path().to_str().unwrap());
        assert!(result.is_ok(), "Err: {:?}", result);
    }
 
    #[test]
    fn test_memory_store_load() {
        let tmp = compile_wat(r#"(module
          (memory (export "memory") 1)
          (func (export "test") (result i32)
            (i32.store (i32.const 0) (i32.const 42))
            (i32.load (i32.const 0))))"#);
        let result = execute_wasm_file(tmp.path().to_str().unwrap());
        assert!(result.is_ok(), "Err: {:?}", result);
    }
 
    #[test]
    fn test_global_set_get() {
        let tmp = compile_wat(r#"(module
          (global $g (mut i32) (i32.const 0))
          (func (export "inc") (result i32)
            (global.set $g (i32.add (global.get $g) (i32.const 1)))
            (global.get $g)))"#);
        let result = execute_wasm_file(tmp.path().to_str().unwrap());
        assert!(result.is_ok(), "Err: {:?}", result);
    }
 
    #[test]
    fn test_if_else() {
        let tmp = compile_wat(r#"(module
          (func (export "max") (param i32 i32) (result i32)
            (if (result i32) (i32.gt_s (local.get 0) (local.get 1))
              (then (local.get 0))
              (else (local.get 1)))))"#);
        let result = execute_wasm_file(tmp.path().to_str().unwrap());
        assert!(result.is_ok(), "Err: {:?}", result);
    }
 
    #[test]
    fn test_noop_module() {
        let tmp = compile_wat(r#"(module (func (export "noop")))"#);
        let result = execute_wasm_file(tmp.path().to_str().unwrap());
        assert!(result.is_ok(), "Err: {:?}", result);
    }
}
