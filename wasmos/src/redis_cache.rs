//! Optional Redis L2 cache layer.
//!
//! When `REDIS_URL` is not set `from_env()` returns `None` and every caller
//! gracefully skips the Redis path.  The app works fully without Redis — it
//! just operates in moka-only (single-pod) mode.

use serde_json::Value;
use std::time::Duration;

/// A minimal Redis client wrapper.  Currently a no-op stub: returns `None`
/// on every read and silently drops writes.  Replace the body of each method
/// with real `redis` / `deadpool-redis` calls once the Redis dependency is
/// added to Cargo.toml.
pub struct RedisCache {
    /// Connection URL stored for future use when a real client is wired in.
    #[allow(dead_code)]
    url: String,
}

impl RedisCache {
    /// Attempt to build a `RedisCache` from the `REDIS_URL` environment
    /// variable.  Returns `None` when the variable is unset — callers treat
    /// `None` as "Redis unavailable, fall back to L1 only".
    pub async fn from_env() -> Option<Self> {
        let url = std::env::var("REDIS_URL").ok()?;
        if url.is_empty() {
            return None;
        }
        // TODO: open a real connection pool here and return None on failure.
        Some(Self { url })
    }

    /// Retrieve a JSON value by key.  Returns `None` on miss or error.
    pub async fn get(&self, _key: &str) -> Option<Value> {
        // Stub: always a miss until a real Redis client is wired in.
        None
    }

    /// Store a JSON value with a TTL.  Errors are swallowed — L1 is the
    /// source of truth and Redis is best-effort.
    pub async fn set(&self, _key: &str, _value: &Value, _ttl: Duration) {
        // Stub: no-op.
    }

    /// Delete a single key.
    pub async fn delete(&self, _key: &str) {
        // Stub: no-op.
    }

    /// Delete all keys that begin with `prefix`.  Used for bulk invalidation
    /// (e.g. "tasks:" to clear every tasks-related cache entry).
    pub async fn delete_prefix(&self, _prefix: &str) {
        // Stub: no-op.
    }
}