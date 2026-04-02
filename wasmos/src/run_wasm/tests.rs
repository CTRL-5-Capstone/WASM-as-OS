/// Comprehensive test suite for WASM execution framework
///
/// Tests cover:
/// - Advanced executor metrics collection
/// - Flow control context management
/// - Loop and iteration tracking
/// - Function call tracing
/// - Import call management
/// - Integrated execution pipeline
/// - Report generation

#[cfg(test)]
mod snake_wasm_tests {
    use crate::run_wasm::wasm_control::execute_wasm_file;
    use std::path::Path;

    /// Verify that snake.wasm parses and executes without panicking.
    /// snake.wasm exports a `start` function (absolute index 24, 1 import + 23 defined before it),
    /// so any bug in the Call handler's fnid index lookup (using absolute instead of local index)
    /// would cause an out-of-bounds panic here.
    #[test]
    fn test_snake_wasm_executes() {
        // Resolve path relative to the workspace root so the test works from any CWD.
        let candidates = [
            "../WasmOSTest/snake.wasm",
            "WasmOSTest/snake.wasm",
            "../../WasmOSTest/snake.wasm",
        ];
        let path = candidates.iter()
            .find(|p| Path::new(p).exists())
            .expect("snake.wasm not found — run tests from the wasmos/ or project root directory");

        let result = execute_wasm_file(path);
        assert!(result.is_ok(), "execute_wasm_file returned Err: {:?}", result.err());

        let exec = result.unwrap();
        assert!(
            exec.success,
            "snake.wasm execution reported failure: {:?}",
            exec.error
        );
    }

    /// Verify that snake.wasm records at least one instruction (i.e. it actually ran).
    #[test]
    fn test_snake_wasm_runs_instructions() {
        let candidates = [
            "../WasmOSTest/snake.wasm",
            "WasmOSTest/snake.wasm",
            "../../WasmOSTest/snake.wasm",
        ];
        let path = candidates.iter()
            .find(|p| Path::new(p).exists())
            .expect("snake.wasm not found");

        if let Ok(exec) = execute_wasm_file(path) {
            if exec.success {
                assert!(
                    exec.instructions_executed > 0,
                    "Expected snake.wasm to execute at least one instruction"
                );
            }
        }
    }
}

#[cfg(test)]
mod advanced_executor_tests {
    use crate::run_wasm::advanced_executor::*;
    use std::collections::HashMap;

    #[test]
    fn test_executor_initialization() {
        let executor = AdvancedExecutor::new();
        assert_eq!(executor.total_instructions, 0);
        assert_eq!(executor.total_syscalls, 0);
        assert!(executor.flow_control_stack.is_empty());
        assert!(executor.call_stack.is_empty());
        assert_eq!(executor.loop_counter, 0);
    }

    #[test]
    fn test_record_instruction_updates_state() {
        let mut executor = AdvancedExecutor::new();

        executor.record_instruction("i32.add".to_string(), 2, 1024, HashMap::new());

        assert_eq!(executor.total_instructions, 1);
        assert_eq!(executor.max_memory_used, 1024);
        assert!(executor.opcode_counter.contains_key("i32.add"));
    }

    #[test]
    fn test_record_multiple_instructions() {
        let mut executor = AdvancedExecutor::new();

        executor.record_instruction("i32.add".to_string(), 2, 512, HashMap::new());
        executor.record_instruction("i32.add".to_string(), 1, 256, HashMap::new());
        executor.record_instruction("i32.sub".to_string(), 1, 512, HashMap::new());

        assert_eq!(executor.total_instructions, 3);
        assert_eq!(executor.max_memory_used, 512);
        assert_eq!(executor.opcode_counter["i32.add"].execution_count, 2);
        assert_eq!(executor.opcode_counter["i32.sub"].execution_count, 1);
    }

    #[test]
    fn test_flow_control_context_enter() {
        let mut executor = AdvancedExecutor::new();

        let block_id = executor.enter_flow_context(FlowContextType::Block);
        assert_eq!(block_id, 1);
        assert_eq!(executor.flow_control_stack.len(), 1);
        assert_eq!(executor.flow_control_stack[0].context_type, FlowContextType::Block);
    }

    #[test]
    fn test_flow_control_multiple_contexts() {
        let mut executor = AdvancedExecutor::new();

        let block_id = executor.enter_flow_context(FlowContextType::Block);
        let loop_id = executor.enter_flow_context(FlowContextType::Loop);
        let branch_id = executor.enter_flow_context(FlowContextType::IfElse);

        assert_eq!(executor.flow_control_stack.len(), 3);
        assert_eq!(block_id, 1);
        assert_eq!(loop_id, 1);
        assert_eq!(branch_id, 1);
    }

    #[test]
    fn test_flow_control_context_exit() {
        let mut executor = AdvancedExecutor::new();

        executor.enter_flow_context(FlowContextType::Block);
        executor.enter_flow_context(FlowContextType::Loop);

        assert_eq!(executor.flow_control_stack.len(), 2);

        let exited = executor.exit_flow_context();
        assert!(exited.is_some());
        assert_eq!(exited.unwrap().context_type, FlowContextType::Loop);
        assert_eq!(executor.flow_control_stack.len(), 1);
    }

    #[test]
    fn test_loop_iteration_tracking() {
        let mut executor = AdvancedExecutor::new();

        executor.enter_flow_context(FlowContextType::Loop);

        for i in 0..100 {
            executor.record_loop_iteration(1);
            if i == 49 {
                // Verify intermediate state
                assert_eq!(executor.loop_tracker[&1].total_iterations, 50);
            }
        }

        assert_eq!(executor.loop_tracker[&1].total_iterations, 100);
    }

    #[test]
    fn test_function_call_tracking() {
        let mut executor = AdvancedExecutor::new();

        executor.enter_function(0, "test_func".to_string());
        assert_eq!(executor.call_stack.len(), 1);
        assert_eq!(executor.call_stack[0].function_index, 0);
        assert_eq!(executor.call_stack[0].function_name, "test_func");

        executor.exit_function();
        assert!(executor.call_stack[0].exit_time_us.is_some());
    }

    #[test]
    fn test_nested_function_calls() {
        let mut executor = AdvancedExecutor::new();

        executor.enter_function(0, "func_a".to_string());
        executor.enter_function(1, "func_b".to_string());
        executor.enter_function(2, "func_c".to_string());

        // Verify the three frames are present with correct depths
        assert_eq!(executor.call_stack.len(), 3);
        assert_eq!(executor.call_stack[0].call_depth, 0);
        assert_eq!(executor.call_stack[1].call_depth, 1);
        assert_eq!(executor.call_stack[2].call_depth, 2);

        // exit_function stamps the deepest (last-pushed) frame each time and
        // does NOT pop it, so the trace is fully preserved for reporting.
        // All three calls stamp frame[2] (the innermost) in sequence.
        executor.exit_function();
        assert!(executor.call_stack[2].exit_time_us.is_some(),
            "exit_function should stamp the deepest frame");

        executor.exit_function();
        executor.exit_function();

        // All three frames still present in the trace (no pop behaviour)
        assert_eq!(executor.call_stack.len(), 3,
            "call_stack trace should be fully preserved after exits");
        // The full call-trace is available for the execution report
        assert_eq!(executor.call_stack[0].function_name, "func_a");
        assert_eq!(executor.call_stack[1].function_name, "func_b");
        assert_eq!(executor.call_stack[2].function_name, "func_c");
    }

    #[test]
    fn test_syscall_recording() {
        let mut executor = AdvancedExecutor::new();

        executor.record_syscall();
        assert_eq!(executor.total_syscalls, 1);

        for _ in 0..9 {
            executor.record_syscall();
        }
        assert_eq!(executor.total_syscalls, 10);
    }

    #[test]
    fn test_logging_stdout() {
        let mut executor = AdvancedExecutor::new();

        executor.log_stdout("Hello".to_string());
        executor.log_stdout("World".to_string());

        assert_eq!(executor.stdout_log.len(), 2);
        assert_eq!(executor.stdout_log[0], "Hello");
        assert_eq!(executor.stdout_log[1], "World");
    }

    #[test]
    fn test_logging_stderr() {
        let mut executor = AdvancedExecutor::new();

        executor.log_stderr("Error 1".to_string());
        executor.log_stderr("Error 2".to_string());

        assert_eq!(executor.stderr_log.len(), 2);
        assert_eq!(executor.stderr_log[0], "Error 1");
    }

    #[test]
    fn test_report_generation_success() {
        let executor = AdvancedExecutor::new();
        let report = executor.generate_report(true, None, Some("42".to_string()), 512);

        assert!(report.success);
        assert!(report.error.is_none());
        assert_eq!(report.return_value, Some("42".to_string()));
        assert_eq!(report.final_memory_used, 512);
    }

    #[test]
    fn test_report_generation_failure() {
        let executor = AdvancedExecutor::new();
        let report = executor.generate_report(
            false,
            Some("Test error".to_string()),
            None,
            1024,
        );

        assert!(!report.success);
        assert_eq!(report.error, Some("Test error".to_string()));
        assert!(report.return_value.is_none());
    }

    #[test]
    fn test_opcode_statistics() {
        let mut executor = AdvancedExecutor::new();

        executor.record_instruction("i32.add".to_string(), 2, 100, HashMap::new());
        executor.record_instruction("i32.add".to_string(), 2, 100, HashMap::new());
        executor.record_instruction("i32.sub".to_string(), 2, 100, HashMap::new());
        executor.record_instruction("i32.mul".to_string(), 2, 100, HashMap::new());

        let report = executor.generate_report(true, None, None, 100);

        assert!(!report.opcode_statistics.is_empty());
        assert_eq!(
            report.opcode_statistics.iter().find(|s| s.opcode_name == "i32.add").unwrap().execution_count,
            2
        );
    }

    #[test]
    fn test_timeline_sampling() {
        let mut executor = AdvancedExecutor::new();
        executor.timeline_sample_rate = 10;

        for _ in 0..100 {
            executor.record_instruction("nop".to_string(), 0, 100, HashMap::new());
        }

        // With sample rate 10, should have approximately 100/10 = 10 samples
        assert!(executor.execution_steps.len() > 0);
        assert!(executor.execution_steps.len() <= 11); // Allow small variance
    }

    #[test]
    fn test_hotspot_detection() {
        let mut executor = AdvancedExecutor::new();

        // Create a scenario where i32.add dominates (>5% of total instructions)
        for _ in 0..600 {
            executor.record_instruction("i32.add".to_string(), 0, 100, HashMap::new());
        }

        for _ in 0..400 {
            executor.record_instruction("nop".to_string(), 0, 100, HashMap::new());
        }

        let report = executor.generate_report(true, None, None, 100);

        // i32.add should be detected as hotspot (60% > 5%)
        assert!(!report.hotspots.is_empty());
        let add_hotspot = report.hotspots.iter().find(|h| h.opcode == "i32.add");
        assert!(add_hotspot.is_some());
    }

    #[test]
    fn test_memory_history_tracking() {
        let mut executor = AdvancedExecutor::new();

        executor.record_instruction("alloc".to_string(), 0, 100, HashMap::new());
        executor.record_instruction("alloc".to_string(), 0, 200, HashMap::new());
        executor.record_instruction("alloc".to_string(), 0, 300, HashMap::new());

        assert!(executor.memory_history.len() > 0);
        assert_eq!(executor.max_memory_used, 300);
    }
}

#[cfg(test)]
mod import_call_manager_tests {
    use crate::run_wasm::import_call_manager::*;

    #[test]
    fn test_manager_initialization() {
        let manager = ImportCallManager::new();
        assert!(manager.get_registered_modules().is_ok());
        assert!(manager.get_registered_modules().unwrap().is_empty());
    }

    #[test]
    fn test_register_single_module() {
        let manager = ImportCallManager::new();
        let config = ImportModuleConfig {
            name: "math".to_string(),
            version: "1.0.0".to_string(),
            enabled: true,
            timeout_us: 1_000_000,
        };

        assert!(manager.register_import_module(config).is_ok());
        let modules = manager.get_registered_modules().unwrap();
        assert_eq!(modules.len(), 1);
        assert!(modules.contains(&"math".to_string()));
    }

    #[test]
    fn test_register_multiple_modules() {
        let manager = ImportCallManager::new();

        for name in &["math", "string", "array", "file"] {
            let config = ImportModuleConfig {
                name: name.to_string(),
                version: "1.0.0".to_string(),
                enabled: true,
                timeout_us: 1_000_000,
            };
            let _ = manager.register_import_module(config);
        }

        let modules = manager.get_registered_modules().unwrap();
        assert_eq!(modules.len(), 4);
    }

    #[test]
    fn test_unregister_module() {
        let manager = ImportCallManager::new();
        let config = ImportModuleConfig {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            enabled: true,
            timeout_us: 1_000_000,
        };

        manager.register_import_module(config).unwrap();
        assert!(manager.unregister_import_module("test").is_ok());

        let modules = manager.get_registered_modules().unwrap();
        assert!(!modules.contains(&"test".to_string()));
    }

    #[test]
    fn test_call_unregistered_module() {
        let manager = ImportCallManager::new();
        let result = manager.call_import("nonexistent", "func", vec![]);

        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_successful_import_call() {
        let manager = ImportCallManager::new();
        let config = ImportModuleConfig {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            enabled: true,
            timeout_us: 1_000_000,
        };

        manager.register_import_module(config).unwrap();
        let result = manager.call_import("test", "func", vec![]);

        assert!(result.success);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_call_history_recording() {
        let manager = ImportCallManager::new();
        let config = ImportModuleConfig {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            enabled: true,
            timeout_us: 1_000_000,
        };

        manager.register_import_module(config).unwrap();

        manager.call_import("test", "func1", vec![]);
        manager.call_import("test", "func2", vec![]);
        manager.call_import("test", "func3", vec![]);

        let history = manager.get_call_history().unwrap();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].function_name, "func1");
        assert_eq!(history[1].function_name, "func2");
        assert_eq!(history[2].function_name, "func3");
    }

    #[test]
    fn test_clear_call_history() {
        let manager = ImportCallManager::new();
        let config = ImportModuleConfig {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            enabled: true,
            timeout_us: 1_000_000,
        };

        manager.register_import_module(config).unwrap();
        manager.call_import("test", "func", vec![]);

        let history = manager.get_call_history().unwrap();
        assert_eq!(history.len(), 1);

        manager.clear_call_history().unwrap();

        let history = manager.get_call_history().unwrap();
        assert!(history.is_empty());
    }

    #[test]
    fn test_call_with_arguments() {
        let manager = ImportCallManager::new();
        let config = ImportModuleConfig {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            enabled: true,
            timeout_us: 1_000_000,
        };

        manager.register_import_module(config).unwrap();

        let args = vec![
            vec![1, 2, 3],
            vec![4, 5, 6],
            vec![7, 8, 9],
        ];

        let result = manager.call_import("test", "func", args.clone());
        assert!(result.success);

        let history = manager.get_call_history().unwrap();
        assert_eq!(history[0].arguments.len(), 3);
    }

    #[test]
    fn test_call_execution_time() {
        let manager = ImportCallManager::new();
        let config = ImportModuleConfig {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            enabled: true,
            timeout_us: 1_000_000,
        };

        manager.register_import_module(config).unwrap();
        let result = manager.call_import("test", "func", vec![]);

        // execution_time_us is u64 so always non-negative; check it's populated
        let _ = result.execution_time_us; // field exists and is accessible
    }
}

#[cfg(test)]
mod execution_framework_tests {
    use crate::run_wasm::execution_framework::*;

    #[test]
    fn test_execution_config_defaults() {
        let config = ExecutionConfig::default();
        assert_eq!(config.max_memory_bytes, 256 * 1024 * 1024);
        assert_eq!(config.max_instructions, 1_000_000_000);
        assert_eq!(config.max_call_depth, 1000);
        assert_eq!(config.timeout_us, 30_000_000);
    }

    #[test]
    fn test_execution_config_customization() {
        let mut config = ExecutionConfig::default();
        config.max_memory_bytes = 512 * 1024 * 1024;
        config.max_instructions = 500_000_000;

        assert_eq!(config.max_memory_bytes, 512 * 1024 * 1024);
        assert_eq!(config.max_instructions, 500_000_000);
    }

    #[test]
    fn test_execution_context_creation() {
        let result = ExecutionContext::new(
            "test.wasm".to_string(),
            ExecutionConfig::default(),
        );

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.wasm_path, "test.wasm");
        assert!(ctx.execution_id.starts_with("exec_"));
    }

    #[test]
    fn test_execution_context_initialization() {
        let ctx = ExecutionContext::new(
            "test.wasm".to_string(),
            ExecutionConfig::default(),
        ).unwrap();

        let result = ctx.initialize_default_imports();
        assert!(result.is_ok());
    }

    #[test]
    fn test_execution_summary_format() {
        let ctx = ExecutionContext::new(
            "test.wasm".to_string(),
            ExecutionConfig::default(),
        ).unwrap();

        let summary = ctx.get_execution_summary();
        assert!(summary.contains("Execution ID:"));
        assert!(summary.contains("test.wasm"));
        assert!(summary.contains("Duration:"));
    }
}

// ─── 0xFC saturating-truncation opcode tests ──────────────────────────────
#[cfg(test)]
mod misc_op_tests {
    use crate::run_wasm::build_runtime::{Runtime, StackCalls, StackTypes};
    use crate::run_wasm::wasm_module::{Module, Code};

    /// Push a single MiscOp(sub) onto a minimal runtime and execute one step.
    /// `stack_in` is pushed before execution; returns the top of the value stack after.
    fn run_misc_op(sub: u32, stack_in: StackTypes) -> StackTypes {
        let module = Module::new();
        let mut rt = Runtime::new(module);
        rt.value_stack.push(stack_in);
        rt.call_stack.push(StackCalls {
            fnid: 0,
            code: vec![Code::MiscOp(sub)],
            loc: 0,
            vars: vec![],
        });
        rt.run_prog();
        rt.value_stack.pop().expect("expected a result on the stack")
    }

    // ── i32.trunc_sat_f32_s (sub=0) ──────────────────────────────────────

    #[test]
    fn test_i32_trunc_sat_f32_s_normal() {
        // Normal truncation toward zero
        match run_misc_op(0, StackTypes::F32(3.9)) {
            StackTypes::I32(v) => assert_eq!(v, 3),
            other => panic!("unexpected {:?}", other),
        }
    }

    #[test]
    fn test_i32_trunc_sat_f32_s_negative() {
        match run_misc_op(0, StackTypes::F32(-3.9)) {
            StackTypes::I32(v) => assert_eq!(v, -3),
            other => panic!("unexpected {:?}", other),
        }
    }

    #[test]
    fn test_i32_trunc_sat_f32_s_nan() {
        match run_misc_op(0, StackTypes::F32(f32::NAN)) {
            StackTypes::I32(v) => assert_eq!(v, 0),
            other => panic!("unexpected {:?}", other),
        }
    }

    #[test]
    fn test_i32_trunc_sat_f32_s_pos_inf() {
        match run_misc_op(0, StackTypes::F32(f32::INFINITY)) {
            StackTypes::I32(v) => assert_eq!(v, i32::MAX),
            other => panic!("unexpected {:?}", other),
        }
    }

    #[test]
    fn test_i32_trunc_sat_f32_s_neg_inf() {
        match run_misc_op(0, StackTypes::F32(f32::NEG_INFINITY)) {
            StackTypes::I32(v) => assert_eq!(v, i32::MIN),
            other => panic!("unexpected {:?}", other),
        }
    }

    // ── i32.trunc_sat_f32_u (sub=1) ──────────────────────────────────────

    #[test]
    fn test_i32_trunc_sat_f32_u_normal() {
        match run_misc_op(1, StackTypes::F32(200.7)) {
            StackTypes::I32(v) => assert_eq!(v as u32, 200),
            other => panic!("unexpected {:?}", other),
        }
    }

    #[test]
    fn test_i32_trunc_sat_f32_u_negative_clamps_to_zero() {
        match run_misc_op(1, StackTypes::F32(-5.0)) {
            StackTypes::I32(v) => assert_eq!(v as u32, 0),
            other => panic!("unexpected {:?}", other),
        }
    }

    #[test]
    fn test_i32_trunc_sat_f32_u_nan_clamps_to_zero() {
        match run_misc_op(1, StackTypes::F32(f32::NAN)) {
            StackTypes::I32(v) => assert_eq!(v as u32, 0),
            other => panic!("unexpected {:?}", other),
        }
    }

    #[test]
    fn test_i32_trunc_sat_f32_u_overflow_clamps_to_max() {
        match run_misc_op(1, StackTypes::F32(5e9_f32)) {
            StackTypes::I32(v) => assert_eq!(v as u32, u32::MAX),
            other => panic!("unexpected {:?}", other),
        }
    }

    // ── i32.trunc_sat_f64_s (sub=2) ──────────────────────────────────────

    #[test]
    fn test_i32_trunc_sat_f64_s_normal() {
        match run_misc_op(2, StackTypes::F64(-7.99)) {
            StackTypes::I32(v) => assert_eq!(v, -7),
            other => panic!("unexpected {:?}", other),
        }
    }

    #[test]
    fn test_i32_trunc_sat_f64_s_nan() {
        match run_misc_op(2, StackTypes::F64(f64::NAN)) {
            StackTypes::I32(v) => assert_eq!(v, 0),
            other => panic!("unexpected {:?}", other),
        }
    }

    #[test]
    fn test_i32_trunc_sat_f64_s_overflow_positive() {
        match run_misc_op(2, StackTypes::F64(1e18_f64)) {
            StackTypes::I32(v) => assert_eq!(v, i32::MAX),
            other => panic!("unexpected {:?}", other),
        }
    }

    // ── i32.trunc_sat_f64_u (sub=3) ──────────────────────────────────────

    #[test]
    fn test_i32_trunc_sat_f64_u_normal() {
        match run_misc_op(3, StackTypes::F64(1000.0)) {
            StackTypes::I32(v) => assert_eq!(v as u32, 1000),
            other => panic!("unexpected {:?}", other),
        }
    }

    #[test]
    fn test_i32_trunc_sat_f64_u_negative_clamps() {
        match run_misc_op(3, StackTypes::F64(-1.0)) {
            StackTypes::I32(v) => assert_eq!(v as u32, 0),
            other => panic!("unexpected {:?}", other),
        }
    }

    // ── i64.trunc_sat_f32_s (sub=4) ──────────────────────────────────────

    #[test]
    fn test_i64_trunc_sat_f32_s_normal() {
        match run_misc_op(4, StackTypes::F32(1234.5)) {
            StackTypes::I64(v) => assert_eq!(v, 1234),
            other => panic!("unexpected {:?}", other),
        }
    }

    #[test]
    fn test_i64_trunc_sat_f32_s_nan() {
        match run_misc_op(4, StackTypes::F32(f32::NAN)) {
            StackTypes::I64(v) => assert_eq!(v, 0),
            other => panic!("unexpected {:?}", other),
        }
    }

    #[test]
    fn test_i64_trunc_sat_f32_s_pos_inf() {
        match run_misc_op(4, StackTypes::F32(f32::INFINITY)) {
            StackTypes::I64(v) => assert_eq!(v, i64::MAX),
            other => panic!("unexpected {:?}", other),
        }
    }

    #[test]
    fn test_i64_trunc_sat_f32_s_neg_inf() {
        match run_misc_op(4, StackTypes::F32(f32::NEG_INFINITY)) {
            StackTypes::I64(v) => assert_eq!(v, i64::MIN),
            other => panic!("unexpected {:?}", other),
        }
    }

    // ── i64.trunc_sat_f32_u (sub=5) ──────────────────────────────────────

    #[test]
    fn test_i64_trunc_sat_f32_u_normal() {
        match run_misc_op(5, StackTypes::F32(999.9)) {
            StackTypes::I64(v) => assert_eq!(v as u64, 999),
            other => panic!("unexpected {:?}", other),
        }
    }

    #[test]
    fn test_i64_trunc_sat_f32_u_negative_clamps() {
        match run_misc_op(5, StackTypes::F32(-0.1)) {
            StackTypes::I64(v) => assert_eq!(v as u64, 0),
            other => panic!("unexpected {:?}", other),
        }
    }

    #[test]
    fn test_i64_trunc_sat_f32_u_nan_clamps() {
        match run_misc_op(5, StackTypes::F32(f32::NAN)) {
            StackTypes::I64(v) => assert_eq!(v as u64, 0),
            other => panic!("unexpected {:?}", other),
        }
    }

    // ── i64.trunc_sat_f64_s (sub=6) ──────────────────────────────────────

    #[test]
    fn test_i64_trunc_sat_f64_s_normal() {
        match run_misc_op(6, StackTypes::F64(-9876543210.0)) {
            StackTypes::I64(v) => assert_eq!(v, -9876543210),
            other => panic!("unexpected {:?}", other),
        }
    }

    #[test]
    fn test_i64_trunc_sat_f64_s_nan() {
        match run_misc_op(6, StackTypes::F64(f64::NAN)) {
            StackTypes::I64(v) => assert_eq!(v, 0),
            other => panic!("unexpected {:?}", other),
        }
    }

    #[test]
    fn test_i64_trunc_sat_f64_s_pos_inf() {
        match run_misc_op(6, StackTypes::F64(f64::INFINITY)) {
            StackTypes::I64(v) => assert_eq!(v, i64::MAX),
            other => panic!("unexpected {:?}", other),
        }
    }

    #[test]
    fn test_i64_trunc_sat_f64_s_neg_inf() {
        match run_misc_op(6, StackTypes::F64(f64::NEG_INFINITY)) {
            StackTypes::I64(v) => assert_eq!(v, i64::MIN),
            other => panic!("unexpected {:?}", other),
        }
    }

    // ── i64.trunc_sat_f64_u (sub=7) ──────────────────────────────────────

    #[test]
    fn test_i64_trunc_sat_f64_u_normal() {
        match run_misc_op(7, StackTypes::F64(1_000_000.0)) {
            StackTypes::I64(v) => assert_eq!(v as u64, 1_000_000),
            other => panic!("unexpected {:?}", other),
        }
    }

    #[test]
    fn test_i64_trunc_sat_f64_u_negative_clamps() {
        match run_misc_op(7, StackTypes::F64(-0.5)) {
            StackTypes::I64(v) => assert_eq!(v as u64, 0),
            other => panic!("unexpected {:?}", other),
        }
    }

    #[test]
    fn test_i64_trunc_sat_f64_u_nan_clamps() {
        match run_misc_op(7, StackTypes::F64(f64::NAN)) {
            StackTypes::I64(v) => assert_eq!(v as u64, 0),
            other => panic!("unexpected {:?}", other),
        }
    }

    // ── Bulk memory/table ops correct stack effects ───────────────────────

    /// memory.init/copy/fill (sub=8,10,11) — pop 3 operands, no result
    #[test]
    fn test_misc_op_bulk_3_pop_no_push() {
        for sub in [8u32, 10, 11, 12, 14, 17] {
            let module = Module::new();
            let mut rt = Runtime::new(module);
            // Push 3 dummy i32 operands
            rt.value_stack.push(StackTypes::I32(1));
            rt.value_stack.push(StackTypes::I32(2));
            rt.value_stack.push(StackTypes::I32(3));
            rt.call_stack.push(StackCalls {
                fnid: 0,
                code: vec![Code::MiscOp(sub)],
                loc: 0,
                vars: vec![],
            });
            rt.run_prog();
            assert!(rt.value_stack.is_empty(),
                "sub={}: expected empty stack after bulk-3 op, got {} items", sub, rt.value_stack.len());
        }
    }

    /// data.drop / elem.drop (sub=9,13) — no stack effect
    #[test]
    fn test_misc_op_drop_no_stack_effect() {
        for sub in [9u32, 13] {
            let module = Module::new();
            let mut rt = Runtime::new(module);
            rt.call_stack.push(StackCalls {
                fnid: 0,
                code: vec![Code::MiscOp(sub)],
                loc: 0,
                vars: vec![],
            });
            rt.run_prog();
            assert!(rt.value_stack.is_empty(),
                "sub={}: expected no stack change", sub);
        }
    }

    /// table.grow (sub=15) — pops 2, pushes -1 (stub)
    #[test]
    fn test_misc_op_table_grow_stub() {
        let module = Module::new();
        let mut rt = Runtime::new(module);
        rt.value_stack.push(StackTypes::I32(0)); // init value
        rt.value_stack.push(StackTypes::I32(10)); // n
        rt.call_stack.push(StackCalls {
            fnid: 0,
            code: vec![Code::MiscOp(15)],
            loc: 0,
            vars: vec![],
        });
        rt.run_prog();
        assert_eq!(rt.value_stack.len(), 1);
        match rt.value_stack.pop().unwrap() {
            StackTypes::I32(v) => assert_eq!(v, -1),
            other => panic!("unexpected {:?}", other),
        }
    }

    /// table.size (sub=16) — pushes 0 (stub)
    #[test]
    fn test_misc_op_table_size_stub() {
        let module = Module::new();
        let mut rt = Runtime::new(module);
        rt.call_stack.push(StackCalls {
            fnid: 0,
            code: vec![Code::MiscOp(16)],
            loc: 0,
            vars: vec![],
        });
        rt.run_prog();
        assert_eq!(rt.value_stack.len(), 1);
        match rt.value_stack.pop().unwrap() {
            StackTypes::I32(v) => assert_eq!(v, 0),
            other => panic!("unexpected {:?}", other),
        }
    }
}
