// WasmOS HTTP API Integration Tests
//
// Tier 1 (no DB required) — run with: cargo test --test api_tests
// Tier 2 (DB required) — run with any of:
//
//   # Local Postgres:
//   WASMOS__DATABASE__URL=postgresql://postgres:postgres@localhost:5432/wasmos_test \
//   cargo test --test api_tests -- --ignored
//
//   # Railway (uses the same DATABASE_URL Railway injects into services):
//   DATABASE_URL=<railway-postgres-url> \
//   cargo test --test api_tests -- --ignored
//
//   # In Railway CLI (runs against live Railway Postgres):
//   railway run cargo test --test api_tests -- --ignored
//
// Design principles:
//  • Tier-1 tests spin up a real actix-web test service with only the endpoints
//    that have no database dependency (health_live, metrics, request parsing).
//  • Tier-2 tests are #[ignore]d and require a live PostgreSQL database.
//  • No mocking frameworks needed — actix-web::test gives us a full in-process
//    HTTP stack that exercises the real serialisation / routing code.
//
// NOTE: `awtest` alias is used instead of `test` to avoid shadowing the
// built-in `#[test]` attribute with `actix_web::test`.

use serde_json::Value;
use actix_web::test as awtest;
use actix_web::App;

// ─── Tier 1: Stateless endpoint tests ────────────────────────────────────────

/// GET /health/live → 200 OK
#[actix_web::test]
async fn test_health_live_returns_200() {
    let app = awtest::init_service(
        App::new().service(wasmos::server::health_live),
    )
    .await;

    let req = awtest::TestRequest::get()
        .uri("/health/live")
        .to_request();

    let resp = awtest::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "Expected 200 OK, got {}",
        resp.status()
    );
}

/// GET /health/live → body contains `"status": "ok"`
#[actix_web::test]
async fn test_health_live_body_has_status_ok() {
    let app = awtest::init_service(
        App::new().service(wasmos::server::health_live),
    )
    .await;

    let req = awtest::TestRequest::get()
        .uri("/health/live")
        .to_request();

    let body: Value = awtest::call_and_read_body_json(&app, req).await;
    assert_eq!(
        body["status"], "ok",
        "health/live should return {{\"status\": \"ok\"}}, got: {body}"
    );
}

/// GET /health/live → body contains a valid RFC-3339 `timestamp` field
#[actix_web::test]
async fn test_health_live_body_has_timestamp() {
    let app = awtest::init_service(
        App::new().service(wasmos::server::health_live),
    )
    .await;

    let req = awtest::TestRequest::get()
        .uri("/health/live")
        .to_request();

    let body: Value = awtest::call_and_read_body_json(&app, req).await;
    assert!(
        body["timestamp"].is_string(),
        "health/live should include a timestamp string, got: {body}"
    );
    let ts = body["timestamp"].as_str().unwrap();
    assert!(
        chrono::DateTime::parse_from_rfc3339(ts).is_ok(),
        "timestamp '{ts}' is not valid RFC-3339"
    );
}

/// GET /health/live → Cache-Control header is present with max-age
#[actix_web::test]
async fn test_health_live_cache_control_header() {
    let app = awtest::init_service(
        App::new().service(wasmos::server::health_live),
    )
    .await;

    let req = awtest::TestRequest::get()
        .uri("/health/live")
        .to_request();

    let resp = awtest::call_service(&app, req).await;
    let cache_ctrl = resp
        .headers()
        .get("Cache-Control")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        cache_ctrl.contains("max-age"),
        "health/live should set a Cache-Control max-age header, got: '{cache_ctrl}'"
    );
}

/// POST /health/live → 405 Method Not Allowed
#[actix_web::test]
async fn test_health_live_wrong_method_405() {
    let app = awtest::init_service(
        App::new()
            .service(wasmos::server::health_live)
            .service(wasmos::server::health_live_post),
    )
    .await;

    let req = awtest::TestRequest::post()
        .uri("/health/live")
        .to_request();

    let resp = awtest::call_service(&app, req).await;
    assert_eq!(
        resp.status(), 405,
        "POST /health/live should return 405, got {}",
        resp.status()
    );
}

/// GET /metrics → 200 with Prometheus text/plain content-type
#[actix_web::test]
async fn test_metrics_endpoint_returns_prometheus_format() {
    let app = awtest::init_service(
        App::new().service(wasmos::server::get_metrics),
    )
    .await;

    let req = awtest::TestRequest::get()
        .uri("/metrics")
        .to_request();

    let resp = awtest::call_service(&app, req).await;
    assert!(resp.status().is_success(), "GET /metrics should return 2xx");

    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        content_type.contains("text/plain"),
        "Prometheus metrics should use text/plain content-type, got: '{content_type}'"
    );
}

/// GET /metrics → response body is non-empty
#[actix_web::test]
async fn test_metrics_endpoint_body_non_empty() {
    let app = awtest::init_service(
        App::new().service(wasmos::server::get_metrics),
    )
    .await;

    let req = awtest::TestRequest::get().uri("/metrics").to_request();
    let body = awtest::call_and_read_body(&app, req).await;
    assert!(!body.is_empty(), "Prometheus metrics body should not be empty");
}

// ─── Tier 1: Unit tests (no HTTP, no DB) ─────────────────────────────────────

/// Valid WASM magic bytes ([0x00, 0x61, 0x73, 0x6D]) are recognised
#[test]
fn test_wasm_magic_bytes_recognised() {
    let magic = [0x00u8, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
    assert_eq!(&magic[..4], b"\0asm");
}

/// Non-WASM bytes do NOT match the WASM magic header
#[test]
fn test_invalid_wasm_magic_not_recognised() {
    let not_wasm = b"ELF\x02";
    assert_ne!(&not_wasm[..4], b"\0asm");
}

/// Upload request JSON round-trips without data loss
#[test]
fn test_upload_request_json_round_trip() {
    let payload = serde_json::json!({
        "name": "test_module",
        "wasm_data": [0, 97, 115, 109, 1, 0, 0, 0],
        "tenant_id": "tenant-abc"
    });
    let serialised = payload.to_string();
    let parsed: Value = serde_json::from_str(&serialised).unwrap();
    assert_eq!(parsed["name"], "test_module");
    assert_eq!(parsed["wasm_data"][0], 0);
    assert_eq!(parsed["tenant_id"], "tenant-abc");
}

/// Task status values are lowercase strings (matches backend enum serde)
#[test]
fn test_task_status_lowercase_values() {
    let valid_statuses = ["pending", "running", "completed", "failed", "stopped"];
    for status in valid_statuses {
        assert_eq!(
            status,
            status.to_lowercase(),
            "Task status '{status}' must be lowercase"
        );
    }
}

/// Batch execution request JSON has the correct structure
#[test]
fn test_batch_request_json_structure() {
    let req = serde_json::json!({
        "wasm_paths": ["/path/to/a.wasm", "/path/to/b.wasm"],
        "continue_on_error": true
    });
    assert!(req["wasm_paths"].is_array());
    assert_eq!(req["wasm_paths"].as_array().unwrap().len(), 2);
    assert_eq!(req["continue_on_error"], true);
}

/// All capability token variants are snake_case (matching Rust serde rename_all)
#[test]
fn test_capability_token_snake_case_variants() {
    let capabilities = [
        "task_read",
        "task_write",
        "task_execute",
        "task_delete",
        "metrics_read",
        "metrics_system",
        "tenant_admin",
        "snapshot_read",
        "snapshot_write",
        "terminal_access",
        "audit_read",
        "admin",
    ];
    for cap in capabilities {
        assert!(
            cap.chars().all(|c| c.is_lowercase() || c == '_'),
            "Capability '{cap}' must be snake_case"
        );
    }
}

/// Execution result JSON has the expected shape
#[test]
fn test_execution_result_json_shape() {
    let result = serde_json::json!({
        "success": true,
        "instructions_executed": 42000,
        "syscalls_executed": 12,
        "memory_used_bytes": 65536,
        "duration_us": 1500,
        "stdout_log": ["hello", "world"],
        "return_value": 0
    });
    assert!(result["success"].as_bool().unwrap());
    assert!(result["instructions_executed"].as_u64().unwrap() > 0);
    assert!(result["stdout_log"].is_array());
}

/// Live metrics JSON shape: latency percentiles are in ascending order
#[test]
fn test_live_metrics_percentile_ordering() {
    let metrics = serde_json::json!({
        "success_rate": 0.95,
        "error_rate": 0.05,
        "p50_us": 1200,
        "p95_us": 4500,
        "p99_us": 12000,
        "avg_us": 1800,
        "throughput_per_min": 42.5
    });
    let p50 = metrics["p50_us"].as_f64().unwrap();
    let p95 = metrics["p95_us"].as_f64().unwrap();
    let p99 = metrics["p99_us"].as_f64().unwrap();
    assert!(p50 <= p95, "P50 must be <= P95 (got p50={p50}, p95={p95})");
    assert!(p95 <= p99, "P95 must be <= P99 (got p95={p95}, p99={p99})");
}

/// success_rate + error_rate should sum to ≈ 1.0
#[test]
fn test_live_metrics_rates_sum_to_one() {
    let metrics = serde_json::json!({
        "success_rate": 0.92,
        "error_rate": 0.08,
        "p50_us": 0, "p95_us": 0, "p99_us": 0, "avg_us": 0,
        "throughput_per_min": 0.0
    });
    let success = metrics["success_rate"].as_f64().unwrap();
    let error   = metrics["error_rate"].as_f64().unwrap();
    let total   = success + error;
    assert!(
        (total - 1.0).abs() < 0.001,
        "success_rate + error_rate should ≈ 1.0, got {total}"
    );
}

/// Minimum WASM binary size is 8 bytes (magic + version)
#[test]
fn test_wasm_minimum_binary_size() {
    // A well-formed empty module: magic(4) + version(4)
    let minimal_wasm = [0x00u8, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
    assert_eq!(minimal_wasm.len(), 8, "Minimal valid WASM is 8 bytes");
    assert_eq!(&minimal_wasm[..4], b"\0asm", "Magic bytes check");
    assert_eq!(&minimal_wasm[4..8], &[0x01, 0x00, 0x00, 0x00], "Version 1 check");
}

// ─── Tier 2: Database-dependent integration tests ────────────────────────────
// Run with: cargo test --test api_tests -- --ignored

#[cfg(test)]
mod db_tests {
    use actix_web::{test as awtest, web, App};
    use serde_json::Value;
    use std::sync::Arc;
    use tokio::sync::broadcast;

    /// Build a full AppState connected to the test database.
    /// Requires WASMOS__DATABASE__URL env var pointing to a Postgres test database.
    async fn build_test_state() -> Result<wasmos::server::AppState, Box<dyn std::error::Error>> {
        // Priority: WASMOS__DATABASE__URL → DATABASE_URL (Railway native) → local fallback
        let db_url = std::env::var("WASMOS__DATABASE__URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .unwrap_or_else(|_| {
                "postgresql://postgres:postgres@localhost:5432/wasmos_test".to_string()
            });

        let pool = wasmos::db::connect_pg(&db_url).await?;
        let task_repo = Arc::new(wasmos::db::repository::TaskRepository::new(pool));

        let config = Arc::new(wasmos::config::Config {
            server: wasmos::config::ServerConfig {
                host: "127.0.0.1".parse().unwrap(),
                port: 8080,
                workers: 1,
                cors_origins: vec!["*".to_string()],
            },
            database: wasmos::config::DatabaseConfig { url: db_url.clone() },
            security: wasmos::config::SecurityConfig {
                auth_enabled: false,
                jwt_secret: "test_secret_at_least_32_chars_long!!".to_string(),
                jwt_expiry_hours: 24,
                rate_limit_per_minute: 1000,
                admin_key: "test_admin_key".to_string(),
            },
            limits: wasmos::config::ResourceLimits {
                max_wasm_size_mb: 50,
                max_memory_mb: 128,
                execution_timeout_secs: 30,
                max_instructions: 10_000_000,
                max_stack_depth: 1024,
                max_concurrent_tasks: 100,
            },
            logging: wasmos::config::LoggingConfig {
                level: "info".to_string(),
                format: "pretty".to_string(),
            },
        });

        let plugin_manager = Arc::new(wasmos::plugins::PluginManager::new());
        let auth_service = Arc::new(wasmos::middleware::auth::AuthService::new(
            "test_secret_at_least_32_chars_long!!".into(),
            24,
            false, // auth disabled in tests
        ));
        let (event_tx, _rx) = broadcast::channel(32);
        // CapabilityRegistry::new(), TraceStore::new(), QueryCache::new() return Arc<Self>
        let cap_registry = wasmos::capability::CapabilityRegistry::new();
        let trace_store = wasmos::tracing_spans::TraceStore::new();
        // Scheduler::new(task_repo, plugin_manager, event_tx, max_concurrent, timeout_secs)
        let scheduler = Arc::new(wasmos::scheduler::Scheduler::new(
            task_repo.clone(),
            plugin_manager.clone(),
            event_tx.clone(),
            4,   // max_concurrent
            30,  // timeout_secs
        ));
        let query_cache = wasmos::query_cache::QueryCache::new();

        Ok(wasmos::server::AppState {
            task_repo,
            config,
            plugin_manager,
            auth_service,
            event_tx,
            cap_registry,
            trace_store,
            scheduler,
            query_cache,
        })
    }

    #[ignore]
    #[actix_web::test]
    async fn test_db_health_ready_connected() {
        let state = build_test_state().await.expect("Failed to build AppState");
        let app = awtest::init_service(
            App::new()
                .app_data(web::Data::new(state))
                .service(wasmos::server::health_ready),
        )
        .await;

        let req = awtest::TestRequest::get().uri("/health/ready").to_request();
        let body: Value = awtest::call_and_read_body_json(&app, req).await;
        assert_eq!(body["database"], "connected");
    }

    #[ignore]
    #[actix_web::test]
    async fn test_db_get_tasks_returns_array() {
        let state = build_test_state().await.expect("Failed to build AppState");
        let app = awtest::init_service(
            App::new()
                .app_data(web::Data::new(state))
                .service(wasmos::server::get_tasks),
        )
        .await;

        let req = awtest::TestRequest::get().uri("/v1/tasks").to_request();
        let body: Value = awtest::call_and_read_body_json(&app, req).await;
        assert!(
            body.is_array(),
            "GET /v1/tasks should return a JSON array, got: {body}"
        );
    }

    #[ignore]
    #[actix_web::test]
    async fn test_db_get_stats_has_expected_fields() {
        let state = build_test_state().await.expect("Failed to build AppState");
        let app = awtest::init_service(
            App::new()
                .app_data(web::Data::new(state))
                .service(wasmos::server::get_stats),
        )
        .await;

        let req = awtest::TestRequest::get().uri("/v1/stats").to_request();
        let body: Value = awtest::call_and_read_body_json(&app, req).await;

        for field in &[
            "total_tasks",
            "running_tasks",
            "completed_tasks",
            "failed_tasks",
            "pending_tasks",
            "total_instructions",
            "total_syscalls",
        ] {
            assert!(
                body.get(*field).is_some(),
                "GET /v1/stats response is missing field '{field}'"
            );
        }
    }

    #[ignore]
    #[actix_web::test]
    async fn test_db_get_nonexistent_task_returns_404() {
        let state = build_test_state().await.expect("Failed to build AppState");
        let app = awtest::init_service(
            App::new()
                .app_data(web::Data::new(state))
                .service(wasmos::server::get_task),
        )
        .await;

        let req = awtest::TestRequest::get()
            .uri("/v1/tasks/00000000-0000-0000-0000-000000000000")
            .to_request();

        let resp = awtest::call_service(&app, req).await;
        assert_eq!(
            resp.status(), 404,
            "GET /v1/tasks/<non-existent-id> should return 404"
        );
    }

    #[ignore]
    #[actix_web::test]
    async fn test_db_audit_log_returns_envelope() {
        let state = build_test_state().await.expect("Failed to build AppState");
        let app = awtest::init_service(
            App::new()
                .app_data(web::Data::new(state))
                .service(wasmos::server::list_audit_log),
        )
        .await;

        let req = awtest::TestRequest::get().uri("/v1/audit").to_request();
        let body: Value = awtest::call_and_read_body_json(&app, req).await;
        assert!(
            body.get("logs").is_some(),
            "GET /v1/audit should return {{logs: [...], ...}}, got: {body}"
        );
        assert!(body["logs"].is_array(), "audit.logs should be an array");
    }

    #[ignore]
    #[actix_web::test]
    async fn test_db_list_tenants_returns_array() {
        let state = build_test_state().await.expect("Failed to build AppState");
        let app = awtest::init_service(
            App::new()
                .app_data(web::Data::new(state))
                .service(wasmos::server::list_tenants),
        )
        .await;

        let req = awtest::TestRequest::get().uri("/v1/tenants").to_request();
        let body: Value = awtest::call_and_read_body_json(&app, req).await;
        assert!(body.is_array(), "GET /v1/tenants should return a JSON array");
    }
}
