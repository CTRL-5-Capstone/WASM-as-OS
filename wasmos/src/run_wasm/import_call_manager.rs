/// Import Call Manager for WASM Outbound Execution
///
/// This module manages outbound imports and external function calls,
/// allowing WASM modules to invoke host functions and handle complex operations.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

/// Result of an import function execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub success: bool,
    pub return_value: Option<Vec<u8>>,
    pub error: Option<String>,
    pub execution_time_us: u64,
}

/// Signature of an import function
#[allow(dead_code)]
pub type ImportFunction = Box<dyn Fn(&[u8]) -> ImportResult + Send + Sync>;

/// Import call metadata
#[derive(Debug, Clone, Serialize)]
pub struct ImportCallMetadata {
    pub module_name: String,
    pub function_name: String,
    pub call_count: u64,
    pub total_execution_time_us: u64,
    pub last_execution_time_us: u64,
    pub success_count: u64,
    pub error_count: u64,
    pub last_error: Option<String>,
}

/// Configuration for an import module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportModuleConfig {
    pub name: String,
    pub version: String,
    pub enabled: bool,
    pub timeout_us: u64,
}

/// Import call context for execution
#[derive(Debug, Clone, Serialize)]
pub struct ImportCallContext {
    pub module_name: String,
    pub function_name: String,
    pub arguments: Vec<Vec<u8>>,
    pub timestamp_us: u64,
    pub call_id: u64,
}

/// Manages import calls and outbound execution
pub struct ImportCallManager {
    imports: Arc<Mutex<HashMap<String, Arc<Mutex<ImportCallMetadata>>>>>,
    call_counter: Arc<Mutex<u64>>,
    call_history: Arc<Mutex<Vec<ImportCallContext>>>,
    max_history_size: usize,
}

impl Default for ImportCallManager {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl ImportCallManager {
    /// Create a new import call manager
    pub fn new() -> Self {
        Self {
            imports: Arc::new(Mutex::new(HashMap::new())),
            call_counter: Arc::new(Mutex::new(0)),
            call_history: Arc::new(Mutex::new(Vec::new())),
            max_history_size: 10000,
        }
    }

    /// Register an import module
    pub fn register_import_module(&self, config: ImportModuleConfig) -> Result<(), String> {
        let metadata = ImportCallMetadata {
            module_name: config.name.clone(),
            function_name: String::new(),
            call_count: 0,
            total_execution_time_us: 0,
            last_execution_time_us: 0,
            success_count: 0,
            error_count: 0,
            last_error: None,
        };

        self.imports
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?
            .insert(config.name, Arc::new(Mutex::new(metadata)));

        Ok(())
    }

    /// Unregister an import module
    pub fn unregister_import_module(&self, module_name: &str) -> Result<(), String> {
        self.imports
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?
            .remove(module_name);
        Ok(())
    }

    /// Execute an import call
    pub fn call_import(
        &self,
        module_name: &str,
        function_name: &str,
        arguments: Vec<Vec<u8>>,
    ) -> ImportResult {
        let start = std::time::Instant::now();

        // Check if module is registered
        let imports = match self.imports.lock() {
            Ok(i) => i,
            Err(e) => {
                return ImportResult {
                    success: false,
                    return_value: None,
                    error: Some(format!("Lock acquisition failed: {}", e)),
                    execution_time_us: start.elapsed().as_micros() as u64,
                }
            }
        };

        if !imports.contains_key(module_name) {
            return ImportResult {
                success: false,
                return_value: None,
                error: Some(format!("Import module not registered: {}", module_name)),
                execution_time_us: start.elapsed().as_micros() as u64,
            };
        }

        drop(imports);

        // Record the call in history
        if let Ok(mut counter) = self.call_counter.lock() {
            *counter += 1;
            let call_id = *counter;

            let context = ImportCallContext {
                module_name: module_name.to_string(),
                function_name: function_name.to_string(),
                arguments: arguments.clone(),
                timestamp_us: start.elapsed().as_micros() as u64,
                call_id,
            };

            if let Ok(mut history) = self.call_history.lock() {
                history.push(context);
                if history.len() > self.max_history_size {
                    history.remove(0);
                }
            }
        }

        // Simulate execution (placeholder for actual import dispatch)
        let execution_time = start.elapsed().as_micros() as u64;

        ImportResult {
            success: true,
            return_value: None,
            error: None,
            execution_time_us: execution_time,
        }
    }

    /// Get import statistics
    pub fn get_import_stats(&self, module_name: &str) -> Result<Option<ImportCallMetadata>, String> {
        self.imports
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?
            .get(module_name)
            .map(|m| {
                m.lock()
                    .map(|metadata| metadata.clone())
                    .map_err(|e| format!("Failed to get metadata: {}", e))
            })
            .transpose()
    }

    /// Get all import modules
    pub fn get_registered_modules(&self) -> Result<Vec<String>, String> {
        Ok(self.imports
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?
            .keys()
            .cloned()
            .collect::<Vec<_>>())
    }

    /// Get call history
    pub fn get_call_history(&self) -> Result<Vec<ImportCallContext>, String> {
        Ok(self.call_history
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?
            .clone())
    }

    /// Clear call history
    pub fn clear_call_history(&self) -> Result<(), String> {
        self.call_history
            .lock()
            .map_err(|e| format!("Failed to acquire lock: {}", e))?
            .clear();
        Ok(())
    }
}

/// Built-in import modules for common operations
#[allow(dead_code)]
pub mod builtin_imports {
    use super::*;

    /// Math operations import module
    pub struct MathImports;

    impl MathImports {
        pub fn register(manager: &ImportCallManager) -> Result<(), String> {
            manager.register_import_module(ImportModuleConfig {
                name: "math".to_string(),
                version: "1.0.0".to_string(),
                enabled: true,
                timeout_us: 1_000_000,
            })
        }

        pub fn sqrt(value: f64) -> f64 {
            value.sqrt()
        }

        pub fn pow(base: f64, exp: f64) -> f64 {
            base.powf(exp)
        }

        pub fn sin(value: f64) -> f64 {
            value.sin()
        }

        pub fn cos(value: f64) -> f64 {
            value.cos()
        }

        pub fn tan(value: f64) -> f64 {
            value.tan()
        }

        pub fn log(value: f64, base: f64) -> f64 {
            value.log(base)
        }
    }

    /// String operations import module
    pub struct StringImports;

    impl StringImports {
        pub fn register(manager: &ImportCallManager) -> Result<(), String> {
            manager.register_import_module(ImportModuleConfig {
                name: "string".to_string(),
                version: "1.0.0".to_string(),
                enabled: true,
                timeout_us: 1_000_000,
            })
        }

        pub fn concat(s1: &str, s2: &str) -> String {
            format!("{}{}", s1, s2)
        }

        pub fn substring(s: &str, start: usize, end: usize) -> String {
            s.chars()
                .skip(start)
                .take(end - start)
                .collect()
        }

        pub fn length(s: &str) -> usize {
            s.len()
        }

        pub fn reverse(s: &str) -> String {
            s.chars().rev().collect()
        }
    }

    /// Array operations import module
    pub struct ArrayImports;

    impl ArrayImports {
        pub fn register(manager: &ImportCallManager) -> Result<(), String> {
            manager.register_import_module(ImportModuleConfig {
                name: "array".to_string(),
                version: "1.0.0".to_string(),
                enabled: true,
                timeout_us: 1_000_000,
            })
        }

        pub fn sort(data: &mut [i32]) {
            data.sort();
        }

        pub fn reverse(data: &mut [i32]) {
            data.reverse();
        }

        pub fn sum(data: &[i32]) -> i64 {
            data.iter().map(|&x| x as i64).sum()
        }

        pub fn max(data: &[i32]) -> Option<i32> {
            data.iter().copied().max()
        }

        pub fn min(data: &[i32]) -> Option<i32> {
            data.iter().copied().min()
        }
    }

    /// File I/O import module
    pub struct FileImports;

    impl FileImports {
        pub fn register(manager: &ImportCallManager) -> Result<(), String> {
            manager.register_import_module(ImportModuleConfig {
                name: "file".to_string(),
                version: "1.0.0".to_string(),
                enabled: true,
                timeout_us: 5_000_000,
            })
        }

        pub fn read_file(path: &str) -> Result<Vec<u8>, String> {
            std::fs::read(path).map_err(|e| e.to_string())
        }

        pub fn write_file(path: &str, data: &[u8]) -> Result<(), String> {
            std::fs::write(path, data).map_err(|e| e.to_string())
        }
    }

    /// Data serialization import module
    pub struct SerializationImports;

    impl SerializationImports {
        pub fn register(manager: &ImportCallManager) -> Result<(), String> {
            manager.register_import_module(ImportModuleConfig {
                name: "serialization".to_string(),
                version: "1.0.0".to_string(),
                enabled: true,
                timeout_us: 1_000_000,
            })
        }

        pub fn json_stringify(data: &serde_json::Value) -> String {
            serde_json::to_string(data).unwrap_or_default()
        }

        pub fn json_parse(json_str: &str) -> Result<serde_json::Value, String> {
            serde_json::from_str(json_str).map_err(|e| e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let manager = ImportCallManager::new();
        assert!(manager.get_registered_modules().is_ok());
    }

    #[test]
    fn test_register_module() {
        let manager = ImportCallManager::new();
        let config = ImportModuleConfig {
            name: "test_module".to_string(),
            version: "1.0.0".to_string(),
            enabled: true,
            timeout_us: 1_000_000,
        };

        let result = manager.register_import_module(config);
        assert!(result.is_ok());

        let modules = manager.get_registered_modules().unwrap();
        assert!(modules.contains(&"test_module".to_string()));
    }

    #[test]
    fn test_call_import() {
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
    }

    #[test]
    fn test_unregistered_module() {
        let manager = ImportCallManager::new();
        let result = manager.call_import("nonexistent", "func", vec![]);
        assert!(!result.success);
    }

    #[test]
    fn test_call_history() {
        let manager = ImportCallManager::new();
        let config = ImportModuleConfig {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            enabled: true,
            timeout_us: 1_000_000,
        };

        manager.register_import_module(config).unwrap();
        let _ = manager.call_import("test", "func1", vec![]);
        let _ = manager.call_import("test", "func2", vec![]);

        let history = manager.get_call_history().unwrap();
        assert_eq!(history.len(), 2);
    }
}
