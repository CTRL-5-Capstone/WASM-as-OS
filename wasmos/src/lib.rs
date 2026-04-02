// lib.rs — re-exports the engine's public API for integration tests and
// external consumers.  The binary entry-point remains in main.rs.

pub mod run_wasm;

// ── Re-export modules needed by integration tests in tests/ ──────────────────
// Each module is already defined in main.rs via `mod xyz;`.
// We mirror them here so tests/ can write `wasmos::server::health_live` etc.

pub mod capability;
pub mod config;
pub mod db;
pub mod error;
pub mod metrics;
pub mod middleware;
pub mod plugins;
pub mod query_cache;
pub mod scheduler;
pub mod server;
pub mod tracing_spans;
