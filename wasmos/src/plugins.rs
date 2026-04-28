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

// ─── In-source tests ─────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};
    use chrono::Utc;
    use uuid::Uuid;
    use crate::db::models::{ExecutionHistory, Task, TaskStatus};
 
    // A test plugin we can read counters back from. PluginManager::register
    // consumes a Box<dyn Plugin>, so we wrap an Arc and keep one out-of-band
    // for assertions.
 
    #[derive(Default)]
    struct CountingPlugin {
        name: String,
        init_calls:     AtomicU64,
        created_calls:  AtomicU64,
        start_calls:    AtomicU64,
        complete_calls: AtomicU64,
        failed_calls:   AtomicU64,
        shutdown_calls: AtomicU64,
    }
 
    impl CountingPlugin {
        fn new(name: &str) -> Arc<Self> {
            Arc::new(Self { name: name.into(), ..Default::default() })
        }
    }
 
    #[async_trait]
    impl Plugin for CountingPlugin {
        fn name(&self) -> &str { &self.name }
        fn version(&self) -> &str { "0.0.1-test" }
 
        async fn on_init(&self) {
            self.init_calls.fetch_add(1, Ordering::SeqCst);
        }
        async fn on_task_created(&self, _: &Task) {
            self.created_calls.fetch_add(1, Ordering::SeqCst);
        }
        async fn on_task_start(&self, _: &str) {
            self.start_calls.fetch_add(1, Ordering::SeqCst);
        }
        async fn on_task_complete(&self, _: &str, _: &ExecutionHistory) {
            self.complete_calls.fetch_add(1, Ordering::SeqCst);
        }
        async fn on_task_failed(&self, _: &str, _: &str) {
            self.failed_calls.fetch_add(1, Ordering::SeqCst);
        }
        async fn on_shutdown(&self) {
            self.shutdown_calls.fetch_add(1, Ordering::SeqCst);
        }
    }
 
    /// Wraps an Arc<P> so it satisfies `Plugin` while letting the test keep
    /// a second Arc reference for read-back.
    struct ArcWrap<P: Plugin>(Arc<P>);
 
    #[async_trait]
    impl<P: Plugin + 'static> Plugin for ArcWrap<P> {
        fn name(&self) -> &str { self.0.name() }
        fn version(&self) -> &str { self.0.version() }
        async fn on_init(&self) { self.0.on_init().await }
        async fn on_task_created(&self, t: &Task) { self.0.on_task_created(t).await }
        async fn on_task_start(&self, id: &str) { self.0.on_task_start(id).await }
        async fn on_task_complete(&self, id: &str, e: &ExecutionHistory) {
            self.0.on_task_complete(id, e).await
        }
        async fn on_task_failed(&self, id: &str, err: &str) {
            self.0.on_task_failed(id, err).await
        }
        async fn on_shutdown(&self) { self.0.on_shutdown().await }
    }
 
    async fn register<P: Plugin + 'static>(manager: &PluginManager, plugin: Arc<P>) {
        let boxed: Box<dyn Plugin> = Box::new(ArcWrap(plugin));
        manager.register(boxed).await;
    }
 
    fn make_task(id: &str) -> Task {
        Task {
            id: id.into(),
            name: "x.wasm".into(),
            path: "/tmp/x.wasm".into(),
            status: TaskStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            file_size_bytes: 0,
            tenant_id: None,
            priority: 5,
        }
    }
 
    fn make_execution(task_id: &str) -> ExecutionHistory {
        ExecutionHistory {
            id: 1,
            execution_id: Uuid::new_v4().to_string(),
            task_id: task_id.into(),
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            duration_us: Some(1_234),
            success: true,
            error: None,
            instructions_executed: 100,
            syscalls_executed: 5,
            memory_used_bytes: 4096,
        }
    }
 
    // ─── Tests ──────────────────────────────────────────────────────────────
 
    #[tokio::test]
    async fn register_invokes_on_init_exactly_once() {
        let manager = PluginManager::new();
        let plugin = CountingPlugin::new("test_init");
 
        register(&manager, plugin.clone()).await;
 
        assert_eq!(plugin.init_calls.load(Ordering::SeqCst), 1);
    }
 
    #[tokio::test]
    async fn trigger_task_created_fans_out_to_every_registered_plugin() {
        let manager = PluginManager::new();
        let p1 = CountingPlugin::new("p1");
        let p2 = CountingPlugin::new("p2");
 
        register(&manager, p1.clone()).await;
        register(&manager, p2.clone()).await;
 
        manager.trigger_task_created(&make_task("t1")).await;
 
        assert_eq!(p1.created_calls.load(Ordering::SeqCst), 1);
        assert_eq!(p2.created_calls.load(Ordering::SeqCst), 1);
    }
 
    #[tokio::test]
    async fn each_lifecycle_hook_dispatches_independently() {
        let manager = PluginManager::new();
        let plugin = CountingPlugin::new("life");
        register(&manager, plugin.clone()).await;
 
        manager.trigger_task_created(&make_task("t1")).await;
        manager.trigger_task_start("t1").await;
        manager.trigger_task_complete("t1", &make_execution("t1")).await;
        manager.trigger_task_failed("t1", "trap").await;
 
        assert_eq!(plugin.created_calls.load(Ordering::SeqCst), 1);
        assert_eq!(plugin.start_calls.load(Ordering::SeqCst), 1);
        assert_eq!(plugin.complete_calls.load(Ordering::SeqCst), 1);
        assert_eq!(plugin.failed_calls.load(Ordering::SeqCst), 1);
    }
 
    #[tokio::test]
    async fn registering_two_plugins_with_distinct_names_keeps_state_independent() {
        let manager = PluginManager::new();
        let p1 = CountingPlugin::new("p1");
        let p2 = CountingPlugin::new("p2");
 
        register(&manager, p1.clone()).await;
        register(&manager, p2.clone()).await;
 
        manager.trigger_task_complete("t1", &make_execution("t1")).await;
 
        assert_eq!(p1.complete_calls.load(Ordering::SeqCst), 1);
        assert_eq!(p2.complete_calls.load(Ordering::SeqCst), 1);
        assert_eq!(p1.failed_calls.load(Ordering::SeqCst), 0);
        assert_eq!(p2.failed_calls.load(Ordering::SeqCst), 0);
    }
 
    #[tokio::test]
    async fn shutdown_fires_on_every_registered_plugin() {
        let manager = PluginManager::new();
        let p1 = CountingPlugin::new("p1");
        let p2 = CountingPlugin::new("p2");
 
        register(&manager, p1.clone()).await;
        register(&manager, p2.clone()).await;
 
        manager.shutdown().await;
 
        assert_eq!(p1.shutdown_calls.load(Ordering::SeqCst), 1);
        assert_eq!(p2.shutdown_calls.load(Ordering::SeqCst), 1);
    }
 
    #[tokio::test]
    async fn triggers_on_empty_manager_are_no_ops_and_do_not_panic() {
        let manager = PluginManager::new();
        manager.trigger_task_created(&make_task("t1")).await;
        manager.trigger_task_start("t1").await;
        manager.trigger_task_complete("t1", &make_execution("t1")).await;
        manager.trigger_task_failed("t1", "boom").await;
        manager.shutdown().await;
    }
 
    // ─── Built-in plugins ───────────────────────────────────────────────────
 
    #[test]
    fn logging_plugin_advertises_canonical_name_and_version() {
        let p = LoggingPlugin;
        assert_eq!(p.name(), "logging");
        assert_eq!(p.version(), "1.0.0");
    }
 
    #[tokio::test]
    async fn metrics_plugin_starts_at_zero() {
        let m = MetricsPlugin::new();
        let (e, f) = m.get_stats().await;
        assert_eq!(e, 0);
        assert_eq!(f, 0);
    }
 
    #[tokio::test]
    async fn metrics_plugin_counts_completions_and_failures_separately() {
        let manager = PluginManager::new();
        let metrics = Arc::new(MetricsPlugin::new());
        register(&manager, metrics.clone()).await;
 
        manager.trigger_task_complete("t1", &make_execution("t1")).await;
        manager.trigger_task_complete("t2", &make_execution("t2")).await;
        manager.trigger_task_failed("t3", "trap").await;
 
        let (executions, failures) = metrics.get_stats().await;
        assert_eq!(executions, 2);
        assert_eq!(failures, 1);
    }
 
    #[tokio::test]
    async fn metrics_plugin_advertises_canonical_name_and_version() {
        let p = MetricsPlugin::new();
        assert_eq!(p.name(), "metrics");
        assert_eq!(p.version(), "1.0.0");
    }
}