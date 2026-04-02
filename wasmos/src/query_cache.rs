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
