use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::db::models::{Task, ExecutionHistory};

#[async_trait]
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    
    async fn on_init(&self) {}
    async fn on_task_created(&self, _task: &Task) {}
    async fn on_task_start(&self, _task_id: &str) {}
    async fn on_task_complete(&self, _task_id: &str, _execution: &ExecutionHistory) {}
    async fn on_task_failed(&self, _task_id: &str, _error: &str) {}
    async fn on_shutdown(&self) {}
}

pub struct PluginManager {
    plugins: Arc<RwLock<HashMap<String, Box<dyn Plugin>>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, plugin: Box<dyn Plugin>) {
        let name = plugin.name().to_string();
        tracing::info!("Registering plugin: {} v{}", name, plugin.version());
        
        plugin.on_init().await;
        
        let mut plugins = self.plugins.write().await;
        plugins.insert(name, plugin);
    }

    pub async fn trigger_task_created(&self, task: &Task) {
        let plugins = self.plugins.read().await;
        for plugin in plugins.values() {
            plugin.on_task_created(task).await;
        }
    }

    pub async fn trigger_task_start(&self, task_id: &str) {
        let plugins = self.plugins.read().await;
        for plugin in plugins.values() {
            plugin.on_task_start(task_id).await;
        }
    }

    pub async fn trigger_task_complete(&self, task_id: &str, execution: &ExecutionHistory) {
        let plugins = self.plugins.read().await;
        for plugin in plugins.values() {
            plugin.on_task_complete(task_id, execution).await;
        }
    }

    pub async fn trigger_task_failed(&self, task_id: &str, error: &str) {
        let plugins = self.plugins.read().await;
        for plugin in plugins.values() {
            plugin.on_task_failed(task_id, error).await;
        }
    }

    pub async fn shutdown(&self) {
        let plugins = self.plugins.read().await;
        for plugin in plugins.values() {
            plugin.on_shutdown().await;
        }
    }
}

// Example plugins

pub struct LoggingPlugin;

#[async_trait]
impl Plugin for LoggingPlugin {
    fn name(&self) -> &str {
        "logging"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn on_task_start(&self, task_id: &str) {
        tracing::info!("📝 Task started: {}", task_id);
    }

    async fn on_task_complete(&self, task_id: &str, execution: &ExecutionHistory) {
        tracing::info!(
            "✅ Task completed: {} (duration: {:?}µs, instructions: {})",
            task_id,
            execution.duration_us,
            execution.instructions_executed
        );
    }

    async fn on_task_failed(&self, task_id: &str, error: &str) {
        tracing::error!("❌ Task failed: {} - {}", task_id, error);
    }
}

pub struct MetricsPlugin {
    total_executions: Arc<RwLock<u64>>,
    total_failures: Arc<RwLock<u64>>,
}

impl MetricsPlugin {
    pub fn new() -> Self {
        Self {
            total_executions: Arc::new(RwLock::new(0)),
            total_failures: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn get_stats(&self) -> (u64, u64) {
        let executions = *self.total_executions.read().await;
        let failures = *self.total_failures.read().await;
        (executions, failures)
    }
}

#[async_trait]
impl Plugin for MetricsPlugin {
    fn name(&self) -> &str {
        "metrics"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn on_task_complete(&self, _task_id: &str, _execution: &ExecutionHistory) {
        let mut count = self.total_executions.write().await;
        *count += 1;
    }

    async fn on_task_failed(&self, _task_id: &str, _error: &str) {
        let mut count = self.total_failures.write().await;
        *count += 1;
    }
}
