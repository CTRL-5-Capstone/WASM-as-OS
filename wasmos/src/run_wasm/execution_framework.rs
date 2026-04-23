/// Integrated Execution Framework
///
/// Combines advanced metrics, flow control, import management, and execution reporting
/// into a unified WASM execution pipeline with comprehensive tracing and analysis.

use crate::run_wasm::advanced_executor::{AdvancedExecutor, AdvancedExecutionReport};
use crate::run_wasm::import_call_manager::ImportCallManager;
use crate::run_wasm::execution_result::ExecutionResult;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;

/// Unified execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    pub max_memory_bytes: u64,
    pub max_instructions: u64,
    pub max_call_depth: u32,
    pub max_loop_iterations: u64,
    pub timeout_us: u64,
    pub enable_tracing: bool,
    pub timeline_sample_rate: usize,
    pub collect_full_history: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: 256 * 1024 * 1024, // 256 MB
            max_instructions: 1_000_000_000,       // 1 billion
            max_call_depth: 1000,
            max_loop_iterations: 100_000_000,
            timeout_us: 30_000_000,                // 30 seconds
            enable_tracing: true,
            timeline_sample_rate: 100,
            collect_full_history: true,
        }
    }
}

/// Comprehensive execution result combining all metrics
#[derive(Debug, Clone, Serialize)]
pub struct IntegratedExecutionResult {
    pub execution_result: ExecutionResult,
    pub advanced_report: AdvancedExecutionReport,
    pub import_stats: Vec<ImportCallStatistics>,
    pub execution_config: ExecutionConfig,
    pub generated_at: String,
    pub execution_id: String,
}

/// Statistics for import calls
#[derive(Debug, Clone, Serialize)]
pub struct ImportCallStatistics {
    pub module_name: String,
    pub total_calls: u64,
    pub successful_calls: u64,
    pub failed_calls: u64,
    pub total_time_us: u64,
    pub average_time_us: f64,
}

/// Execution context for complex WASM operations
#[allow(dead_code)]
pub struct ExecutionContext {
    pub config: ExecutionConfig,
    pub executor: AdvancedExecutor,
    pub import_manager: ImportCallManager,
    pub execution_id: String,
    pub start_time: Instant,
    pub wasm_path: String,
}

#[allow(dead_code)]
impl ExecutionContext {
    /// Create a new execution context
    pub fn new(wasm_path: String, config: ExecutionConfig) -> Result<Self, String> {
        let execution_id = format!(
            "exec_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        );

        let mut executor = AdvancedExecutor::new();
        executor.timeline_sample_rate = config.timeline_sample_rate;

        let import_manager = ImportCallManager::new();

        Ok(ExecutionContext {
            config,
            executor,
            import_manager,
            execution_id,
            start_time: Instant::now(),
            wasm_path,
        })
    }

    /// Initialize default import modules
    pub fn initialize_default_imports(&self) -> Result<(), String> {
        use crate::run_wasm::import_call_manager::builtin_imports::*;

        MathImports::register(&self.import_manager)?;
        StringImports::register(&self.import_manager)?;
        ArrayImports::register(&self.import_manager)?;
        FileImports::register(&self.import_manager)?;
        SerializationImports::register(&self.import_manager)?;

        Ok(())
    }

    /// Execute a WASM module with integrated metrics
    pub fn execute(&mut self) -> Result<IntegratedExecutionResult, String> {
        // Validate WASM file exists
        let path = Path::new(&self.wasm_path);
        if !path.exists() {
            return Err(format!("WASM file not found: {}", self.wasm_path));
        }

        // Execute the WASM module
        let execution_result = crate::run_wasm::execute_wasm_file(&self.wasm_path, None)?;

        // Generate advanced report
        let advanced_report = self.executor.generate_report(
            execution_result.success,
            execution_result.error.clone(),
            execution_result.return_value.clone(),
            execution_result.memory_used_bytes,
        );

        // Collect import statistics
        let import_stats = self.collect_import_statistics()?;

        let result = IntegratedExecutionResult {
            execution_result,
            advanced_report,
            import_stats,
            execution_config: self.config.clone(),
            generated_at: chrono::Local::now().to_rfc3339(),
            execution_id: self.execution_id.clone(),
        };

        Ok(result)
    }

    /// Collect statistics from import calls
    fn collect_import_statistics(&self) -> Result<Vec<ImportCallStatistics>, String> {
        let modules = self.import_manager.get_registered_modules()?;
        let mut stats = Vec::new();

        for module_name in modules {
            if let Some(metadata) = self.import_manager.get_import_stats(&module_name)? {
                let average_time = if metadata.call_count > 0 {
                    metadata.total_execution_time_us as f64 / metadata.call_count as f64
                } else {
                    0.0
                };

                stats.push(ImportCallStatistics {
                    module_name,
                    total_calls: metadata.call_count,
                    successful_calls: metadata.success_count,
                    failed_calls: metadata.error_count,
                    total_time_us: metadata.total_execution_time_us,
                    average_time_us: average_time,
                });
            }
        }

        Ok(stats)
    }

    /// Get execution summary
    pub fn get_execution_summary(&self) -> String {
        format!(
            "Execution ID: {}\nPath: {}\nDuration: {}ms\nInstructions: {}\nSyscalls: {}",
            self.execution_id,
            self.wasm_path,
            self.start_time.elapsed().as_millis(),
            self.executor.total_instructions,
            self.executor.total_syscalls,
        )
    }
}

/// High-level execution dispatcher
pub struct ExecutionDispatcher;

impl ExecutionDispatcher {
    /// Execute a single WASM file
    pub fn execute_file(
        wasm_path: &str,
        config: Option<ExecutionConfig>,
    ) -> Result<IntegratedExecutionResult, String> {
        let config = config.unwrap_or_default();
        let mut context = ExecutionContext::new(wasm_path.to_string(), config)?;
        context.initialize_default_imports()?;
        context.execute()
    }

    /// Execute multiple WASM files in sequence
    pub fn execute_batch(
        wasm_paths: &[&str],
        config: Option<ExecutionConfig>,
    ) -> Result<Vec<IntegratedExecutionResult>, String> {
        let mut results = Vec::new();

        for path in wasm_paths {
            match Self::execute_file(path, config.clone()) {
                Ok(result) => results.push(result),
                Err(e) => {
                    eprintln!("Failed to execute {}: {}", path, e);
                    // Continue with next file
                }
            }
        }

        if results.is_empty() {
            return Err("No files executed successfully".to_string());
        }

        Ok(results)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_config_default() {
        let config = ExecutionConfig::default();
        assert_eq!(config.max_memory_bytes, 256 * 1024 * 1024);
        assert_eq!(config.max_instructions, 1_000_000_000);
    }

    #[test]
    fn test_execution_context_creation() {
        let result = ExecutionContext::new("test.wasm".to_string(), ExecutionConfig::default());
        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.wasm_path, "test.wasm");
    }

    #[test]
    fn test_execution_id_format() {
        let ctx = ExecutionContext::new("test.wasm".to_string(), ExecutionConfig::default()).unwrap();
        assert!(ctx.execution_id.starts_with("exec_"));
    }

    #[test]
    fn test_import_initialization() {
        let ctx = ExecutionContext::new("test.wasm".to_string(), ExecutionConfig::default()).unwrap();
        let result = ctx.initialize_default_imports();
        assert!(result.is_ok());
    }
}
