//! Two-level query result cache: L1 (moka, in-process) + L2 (Redis, optional).
//!
//! ## Tier behaviour
//!
//! | Tier | Backend  | Latency    | Scope        | TTL source   |
//! |------|----------|------------|--------------|--------------|
//! | L1   | moka     | ~100 ns    | single pod   | moka builder |
//! | L2   | Redis    | ~1-2 ms    | all replicas | SET EX arg   |
//!
//! ## Cache operations
//!
//! **GET**: check L1 → if miss, check L2 → if hit, backfill L1.
//! **INSERT**: write L1 then L2 (fire-and-forget on Redis failure).
//! **INVALIDATE**: clear L1 immediately + delete matching Redis keys.
//!
//! The app works correctly without Redis — `RedisCache` is `Option<RedisCache>`
//! so all Redis paths are skipped gracefully when `REDIS_URL` is unset.
//!
//! ## Endpoints currently cached
//!
//! | Cache method            | Endpoint              | L1 TTL | L2 TTL |
//! |-------------------------|-----------------------|--------|--------|
//! | `get/insert_tasks`      | GET /v1/tasks         | 15 s   | 30 s   |
//! | `get/insert_stats`      | GET /v1/stats         | 10 s   | 20 s   |
//! | `get/insert_task`       | GET /v1/tasks/{id}    | 30 s   | 60 s   |
//! | `get/insert_tokens`     | GET /v1/tokens        | 60 s   | 120 s  |
//! | `get/insert_scheduler`  | GET /v1/scheduler/..  | 5 s    | 10 s   |
//! | `get/insert_traces`     | GET /v1/traces        | 5 s    | 10 s   |

use crate::redis_cache::RedisCache;
use moka::future::Cache;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

/// Cache keys are derived from query parameters so different filter
/// combinations each get their own entry.
fn tasks_key(tenant_id: Option<&str>, status: Option<&str>, limit: i64, offset: i64) -> String {
    format!(
        "tasks:t={}|s={}|lim={}|off={}",
        tenant_id.unwrap_or("*"),
        status.unwrap_or("*"),
        limit,
        offset,
    )
}

pub struct QueryCache {
    /// L1: in-process moka caches (fast, per-pod)
    tasks: Cache<String, Value>,
    stats: Cache<String, Value>,
    task: Cache<String, Value>,
    tokens: Cache<String, Value>,
    scheduler: Cache<String, Value>,
    traces: Cache<String, Value>,

    /// L2: optional Redis (shared across replicas)
    redis: Option<RedisCache>,
}

impl QueryCache {
    /// Build a new cache. Attempts to connect to Redis via `REDIS_URL`.
    pub async fn new() -> Arc<Self> {
        let redis = RedisCache::from_env().await;
        if redis.is_some() {
            tracing::info!("QueryCache: Redis L2 cache active");
        } else {
            tracing::info!("QueryCache: moka-only (set REDIS_URL to enable Redis L2)");
        }

        Arc::new(Self {
            tasks: Cache::builder()
                .time_to_live(Duration::from_secs(15))
                .time_to_idle(Duration::from_secs(60))
                .max_capacity(512)
                .build(),
            stats: Cache::builder()
                .time_to_live(Duration::from_secs(10))
                .max_capacity(1)
                .build(),
            task: Cache::builder()
                .time_to_live(Duration::from_secs(30))
                .time_to_idle(Duration::from_secs(120))
                .max_capacity(1024)
                .build(),
            tokens: Cache::builder()
                .time_to_live(Duration::from_secs(60))
                .time_to_idle(Duration::from_secs(300))
                .max_capacity(128)
                .build(),
            scheduler: Cache::builder()
                .time_to_live(Duration::from_secs(5))
                .max_capacity(4)
                .build(),
            traces: Cache::builder()
                .time_to_live(Duration::from_secs(5))
                .time_to_idle(Duration::from_secs(30))
                .max_capacity(256)
                .build(),
            redis,
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
        let key = tasks_key(tenant_id, status, limit, offset);
        // L1 hit
        if let Some(v) = self.tasks.get(&key).await {
            return Some(v);
        }
        // L2 hit — backfill L1
        if let Some(ref r) = self.redis {
            if let Some(v) = r.get(&key).await {
                self.tasks.insert(key, v.clone()).await;
                return Some(v);
            }
        }
        None
    }

    pub async fn insert_tasks(
        &self,
        tenant_id: Option<&str>,
        status: Option<&str>,
        limit: i64,
        offset: i64,
        value: Value,
    ) {
        let key = tasks_key(tenant_id, status, limit, offset);
        self.tasks.insert(key.clone(), value.clone()).await;
        if let Some(ref r) = self.redis {
            r.set(&key, &value, Duration::from_secs(30)).await;
        }
    }

    pub async fn invalidate_tasks(&self) {
        self.tasks.invalidate_all();
        if let Some(ref r) = self.redis {
            r.delete_prefix("tasks:").await;
        }
    }

    // ─── Stats ───────────────────────────────────────────────────────────────

    pub async fn get_stats(&self) -> Option<Value> {
        if let Some(v) = self.stats.get("stats").await {
            return Some(v);
        }
        if let Some(ref r) = self.redis {
            if let Some(v) = r.get("stats").await {
                self.stats.insert("stats".to_string(), v.clone()).await;
                return Some(v);
            }
        }
        None
    }

    pub async fn insert_stats(&self, value: Value) {
        self.stats.insert("stats".to_string(), value.clone()).await;
        if let Some(ref r) = self.redis {
            r.set("stats", &value, Duration::from_secs(20)).await;
        }
    }

    pub async fn invalidate_stats(&self) {
        self.stats.invalidate("stats").await;
        if let Some(ref r) = self.redis {
            r.delete("stats").await;
        }
    }

    // ─── Individual task ─────────────────────────────────────────────────────

    pub async fn get_task(&self, id: &str) -> Option<Value> {
        let key = format!("task:{}", id);
        if let Some(v) = self.task.get(&key).await {
            return Some(v);
        }
        if let Some(ref r) = self.redis {
            if let Some(v) = r.get(&key).await {
                self.task.insert(key, v.clone()).await;
                return Some(v);
            }
        }
        None
    }

    pub async fn insert_task(&self, id: &str, value: Value) {
        let key = format!("task:{}", id);
        self.task.insert(key.clone(), value.clone()).await;
        if let Some(ref r) = self.redis {
            r.set(&key, &value, Duration::from_secs(60)).await;
        }
    }

    pub async fn invalidate_task(&self, id: &str) {
        let key = format!("task:{}", id);
        self.task.invalidate(&key).await;
        if let Some(ref r) = self.redis {
            r.delete(&key).await;
        }
    }

    // ─── Capability tokens ───────────────────────────────────────────────────

    pub async fn get_tokens(&self, scope_key: &str) -> Option<Value> {
        let key = format!("tokens:{}", scope_key);
        if let Some(v) = self.tokens.get(&key).await {
            return Some(v);
        }
        if let Some(ref r) = self.redis {
            if let Some(v) = r.get(&key).await {
                self.tokens.insert(key, v.clone()).await;
                return Some(v);
            }
        }
        None
    }

    pub async fn insert_tokens(&self, scope_key: &str, value: Value) {
        let key = format!("tokens:{}", scope_key);
        self.tokens.insert(key.clone(), value.clone()).await;
        if let Some(ref r) = self.redis {
            r.set(&key, &value, Duration::from_secs(120)).await;
        }
    }

    pub async fn invalidate_tokens(&self) {
        self.tokens.invalidate_all();
        if let Some(ref r) = self.redis {
            r.delete_prefix("tokens:").await;
        }
    }

    // ─── Scheduler status ────────────────────────────────────────────────────

    pub async fn get_scheduler(&self, key: &str) -> Option<Value> {
        let rkey = format!("sched:{}", key);
        if let Some(v) = self.scheduler.get(&rkey).await {
            return Some(v);
        }
        if let Some(ref r) = self.redis {
            if let Some(v) = r.get(&rkey).await {
                self.scheduler.insert(rkey, v.clone()).await;
                return Some(v);
            }
        }
        None
    }

    pub async fn insert_scheduler(&self, key: &str, value: Value) {
        let rkey = format!("sched:{}", key);
        self.scheduler.insert(rkey.clone(), value.clone()).await;
        if let Some(ref r) = self.redis {
            r.set(&rkey, &value, Duration::from_secs(10)).await;
        }
    }

    pub async fn invalidate_scheduler(&self) {
        self.scheduler.invalidate_all();
        if let Some(ref r) = self.redis {
            r.delete_prefix("sched:").await;
        }
    }

    // ─── Traces ──────────────────────────────────────────────────────────────

    pub async fn get_traces(&self, key: &str) -> Option<Value> {
        let rkey = format!("traces:{}", key);
        if let Some(v) = self.traces.get(&rkey).await {
            return Some(v);
        }
        if let Some(ref r) = self.redis {
            if let Some(v) = r.get(&rkey).await {
                self.traces.insert(rkey, v.clone()).await;
                return Some(v);
            }
        }
        None
    }

    pub async fn insert_traces(&self, key: &str, value: Value) {
        let rkey = format!("traces:{}", key);
        self.traces.insert(rkey.clone(), value.clone()).await;
        if let Some(ref r) = self.redis {
            r.set(&rkey, &value, Duration::from_secs(10)).await;
        }
    }

    pub async fn invalidate_traces(&self) {
        self.traces.invalidate_all();
        if let Some(ref r) = self.redis {
            r.delete_prefix("traces:").await;
        }
    }
}
