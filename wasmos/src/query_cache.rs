//! In-process query result cache.
//!
//! Uses `moka` (a high-performance concurrent cache inspired by Caffeine) to
//! reduce redundant database round-trips for hot read paths:
//!
//!   • `/v1/tasks`   — task listing (TTL 15 s)
//!   • `/v1/stats`   — aggregate stats (TTL 10 s)
//!
//! The cache is intentionally short-lived: its purpose is to absorb traffic
//! spikes and concurrent duplicate requests, not to serve stale data
//! indefinitely. All write operations (`upload_task`, `start_task`,
//! `delete_task`, `update_task`) call `invalidate_tasks()` /
//! `invalidate_stats()` so reads following a write always see fresh data.
//!
//! No Redis is required — this is a single-process cache appropriate for the
//! current single-node deployment model. A distributed cache (e.g. Redis) can
//! be layered on later if the service is scaled horizontally.

use moka::future::Cache;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

/// Cache keys are derived from query parameters so different filter
/// combinations each get their own entry.
fn tasks_key(tenant_id: Option<&str>, status: Option<&str>, limit: i64, offset: i64) -> String {
    format!(
        "tasks|t={}|s={}|lim={}|off={}",
        tenant_id.unwrap_or("*"),
        status.unwrap_or("*"),
        limit,
        offset,
    )
}

pub struct QueryCache {
    /// Cached task-list JSON payloads, keyed by query fingerprint.
    tasks: Cache<String, Value>,
    /// Cached stats JSON payload (single entry under key "stats").
    stats: Cache<String, Value>,
}

impl QueryCache {
    /// Build a new cache with production-tuned TTLs.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            tasks: Cache::builder()
                // 15 s TTL: fresh enough for most dashboards, dramatically
                // cuts DB load under concurrent polling.
                .time_to_live(Duration::from_secs(15))
                // Evict entries not accessed for 60 s (saves memory when many
                // unique filter combos were queried once and never again).
                .time_to_idle(Duration::from_secs(60))
                // Maximum 512 distinct task-list query variants cached at once.
                .max_capacity(512)
                .build(),
            stats: Cache::builder()
                .time_to_live(Duration::from_secs(10))
                .max_capacity(1)
                .build(),
        })
    }

    // ─── Task list ───────────────────────────────────────────────────────────

    pub async fn get_tasks(
        &self,
        tenant_id: Option<&str>,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Option<Value> {
        self.tasks
            .get(&tasks_key(tenant_id, status, limit, offset))
            .await
    }

    pub async fn insert_tasks(
        &self,
        tenant_id: Option<&str>,
        status: Option<&str>,
        limit: i64,
        offset: i64,
        value: Value,
    ) {
        self.tasks
            .insert(tasks_key(tenant_id, status, limit, offset), value)
            .await;
    }

    /// Invalidate all task-list cache entries.
    /// Call after any mutation that changes the task list.
    pub async fn invalidate_tasks(&self) {
        self.tasks.invalidate_all();
    }

    // ─── Stats ───────────────────────────────────────────────────────────────

    pub async fn get_stats(&self) -> Option<Value> {
        self.stats.get("stats").await
    }

    pub async fn insert_stats(&self, value: Value) {
        self.stats.insert("stats".to_string(), value).await;
    }

    /// Invalidate the stats cache entry.
    /// Call after any mutation that changes aggregate counts.
    pub async fn invalidate_stats(&self) {
        self.stats.invalidate("stats").await;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
 
    #[tokio::test]
    async fn test_insert_and_get_tasks() {
        let cache = QueryCache::new();
        let data = json!({"tasks": [{"id": "t1"}]});
        cache.insert_tasks(None, None, 10, 0, data.clone()).await;
        let cached = cache.get_tasks(None, None, 10, 0).await;
        assert_eq!(cached.unwrap(), data);
    }
 
    #[tokio::test]
    async fn test_cache_miss_returns_none() {
        let cache = QueryCache::new();
        assert!(cache.get_tasks(None, None, 10, 0).await.is_none());
    }
 
    #[tokio::test]
    async fn test_different_params_are_separate() {
        let cache = QueryCache::new();
        let a = json!({"page": "a"});
        let b = json!({"page": "b"});
        cache.insert_tasks(None, None, 10, 0, a.clone()).await;
        cache.insert_tasks(None, None, 10, 10, b.clone()).await;
        assert_eq!(cache.get_tasks(None, None, 10, 0).await.unwrap(), a);
        assert_eq!(cache.get_tasks(None, None, 10, 10).await.unwrap(), b);
    }
 
    #[tokio::test]
    async fn test_invalidate_tasks_clears_all() {
        let cache = QueryCache::new();
        cache.insert_tasks(None, None, 10, 0, json!({"x": 1})).await;
        cache.insert_tasks(Some("t1"), None, 10, 0, json!({"x": 2})).await;
        cache.invalidate_tasks().await;
        assert!(cache.get_tasks(None, None, 10, 0).await.is_none());
        assert!(cache.get_tasks(Some("t1"), None, 10, 0).await.is_none());
    }
 
    #[tokio::test]
    async fn test_stats_insert_and_get() {
        let cache = QueryCache::new();
        let stats = json!({"total": 42});
        cache.insert_stats(stats.clone()).await;
        assert_eq!(cache.get_stats().await.unwrap(), stats);
    }
 
    #[tokio::test]
    async fn test_stats_miss() {
        let cache = QueryCache::new();
        assert!(cache.get_stats().await.is_none());
    }
 
    #[tokio::test]
    async fn test_invalidate_stats_clears() {
        let cache = QueryCache::new();
        cache.insert_stats(json!({"n": 1})).await;
        cache.invalidate_stats().await;
        assert!(cache.get_stats().await.is_none());
    }
 
    #[tokio::test]
    async fn test_tenant_isolation() {
        let cache = QueryCache::new();
        let d1 = json!({"t": "t1"});
        let d2 = json!({"t": "t2"});
        cache.insert_tasks(Some("t1"), None, 10, 0, d1.clone()).await;
        cache.insert_tasks(Some("t2"), None, 10, 0, d2.clone()).await;
        assert_eq!(cache.get_tasks(Some("t1"), None, 10, 0).await.unwrap(), d1);
        assert_eq!(cache.get_tasks(Some("t2"), None, 10, 0).await.unwrap(), d2);
    }
 
    #[tokio::test]
    async fn test_status_filter_isolation() {
        let cache = QueryCache::new();
        let run = json!({"s": "running"});
        let done = json!({"s": "completed"});
        cache.insert_tasks(None, Some("running"), 10, 0, run.clone()).await;
        cache.insert_tasks(None, Some("completed"), 10, 0, done.clone()).await;
        assert_eq!(cache.get_tasks(None, Some("running"), 10, 0).await.unwrap(), run);
        assert_eq!(cache.get_tasks(None, Some("completed"), 10, 0).await.unwrap(), done);
    }
 
    #[test]
    fn test_tasks_key_format() {
        let key = tasks_key(Some("acme"), Some("running"), 20, 5);
        assert!(key.contains("t=acme"));
        assert!(key.contains("s=running"));
        assert!(key.contains("lim=20"));
        assert!(key.contains("off=5"));
    }
 
    #[test]
    fn test_tasks_key_defaults() {
        let key = tasks_key(None, None, 10, 0);
        assert!(key.contains("t=*"));
        assert!(key.contains("s=*"));
    }
}