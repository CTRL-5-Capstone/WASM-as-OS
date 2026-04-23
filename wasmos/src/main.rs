mod config;
mod capability;
mod db;
mod error;
mod metrics;
mod middleware;
mod query_cache;
mod redis_cache;
mod run_wasm;
mod server;
mod scheduler;
mod tenancy;
mod plugins;
mod tracing_spans;
mod websocket;
use std::path::{Path, PathBuf};
mod telemetry;
mod advanced_execution_endpoints;
fn resolve_web_dir() -> Option<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest_dir.join("../web"),
        manifest_dir.join("web"),
        PathBuf::from("../web"),
        PathBuf::from("web"),
    ];

    for candidate in candidates {
        if candidate.is_dir() {
            tracing::info!("Serving static web from: {}", candidate.display());
            return Some(candidate);
        }
    }

    tracing::warn!(
        "Static web directory not found; skipping static file serving. CARGO_MANIFEST_DIR={}",
        manifest_dir.display()
    );
    None
}

fn configure_static_files(cfg: &mut actix_web::web::ServiceConfig) {
    if let Some(web_dir) = resolve_web_dir() {
        cfg.service(
            actix_files::Files::new("/", web_dir)
                .index_file("index.html")
                .redirect_to_slash_directory()
        );
    }
}
use crate::config::Config;
use crate::db::{connect_pg, repository::TaskRepository};
use crate::middleware::{auth::JwtAuth, logging::RequestId, rate_limit::RateLimiter, security_headers::SecurityHeaders};
use crate::server::AppState;
use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use std::sync::Arc;
use tracing_actix_web::TracingLogger;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Load configuration
    let config = Config::load().expect("Failed to load configuration");
    
    // Initialize logging
    telemetry::init_tracing(&config.logging.level, &config.logging.format);
    
    tracing::info!("Starting WASM-OS...");
    tracing::info!("Configuration loaded: {:?}", config);
    
    // Connect to PostgreSQL (graceful - app runs without DB)
    let pool = match connect_pg(&config.database.url).await {
        Ok(pool) => {
            tracing::info!("Connected to PostgreSQL");
            pool
        }
        Err(e) => {
            tracing::warn!(
                "PostgreSQL unavailable ({}). Starting without database - task persistence disabled.",
                e
            );
            // Return a placeholder pool that will error on use — handlers return 503 gracefully
            // We still need a pool for AppState; create a disconnected one
            sqlx::PgPool::connect_lazy(&config.database.url)
                .expect("Failed to create lazy pool")
        }
    };
    
    // Create repository
    let task_repo = Arc::new(TaskRepository::new(pool));

    // Migrations are run inside connect_pg() via sqlx::migrate!().
    // No additional migration call is needed here.

    // Ensure wasm_files directory exists at startup (upload_task also creates it,
    // but doing it here avoids a race on first upload).
    // Resolution order:
    //   1. WASM_FILES_DIR environment variable (recommended for production)
    //   2. ./wasm_files relative to the current working directory
    //   3. <CARGO_MANIFEST_DIR>/wasm_files (compile-time fallback, dev only)
    let wasm_dir = std::env::var("WASM_FILES_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            let cwd_candidate = std::path::PathBuf::from("wasm_files");
            if cwd_candidate.exists() {
                cwd_candidate
            } else {
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("wasm_files")
            }
        });
    if let Err(e) = tokio::fs::create_dir_all(&wasm_dir).await {
        tracing::warn!("Could not create wasm_files dir: {}", e);
    } else {
        tracing::info!("wasm_files dir ready: {}", wasm_dir.display());
    }

    // Initialize plugin manager and register concrete plugins
    let plugin_manager = Arc::new(plugins::PluginManager::new());
    plugin_manager.register(Box::new(plugins::LoggingPlugin)).await;
    plugin_manager.register(Box::new(plugins::MetricsPlugin::new())).await;

    // Initialize tenant manager with a default tenant
    let mut tenant_mgr = tenancy::TenantManager::new();
    let _default_tenant = tenant_mgr.create_tenant(
        "default".to_string(),
        tenancy::ResourceQuota {
            max_tasks: 1000,
            max_memory_mb: 2048,
            max_cpu_percent: 100,
            max_concurrent_executions: 64,
            max_wasm_size_mb: 256,
        },
    );
    tracing::info!("TenantManager initialised with default tenant");

    // Broadcast channel for real-time WebSocket task events (capacity = 256 events)
    let (event_tx, _event_rx_unused) = tokio::sync::broadcast::channel::<crate::server::TaskEvent>(256);
    let event_tx = Arc::new(event_tx);

    // Initialize capability token registry
    let cap_registry = crate::capability::CapabilityRegistry::new();

    // Initialize two-level query cache (moka L1 + optional Redis L2)
    let query_cache = crate::query_cache::QueryCache::new().await;

    // Initialize distributed trace store
    let trace_store = crate::tracing_spans::TraceStore::new();

    // Build scheduler (shared Arc so AppState can expose status/preempt API)
    let sched = Arc::new(scheduler::Scheduler::new(
        task_repo.clone(),
        plugin_manager.clone(),
        (*event_tx).clone(),
        config.limits.max_concurrent_tasks,
        config.limits.execution_timeout_secs,
    ));

    // Spawn scheduler background task
    {
        let sched_run = sched.clone();
        tokio::spawn(async move { sched_run.run().await });
        tracing::info!(
            "Scheduler started (max_concurrent={}, timeout_secs={}, preemptive=true)",
            config.limits.max_concurrent_tasks,
            config.limits.execution_timeout_secs,
        );
    }

    // Spawn capability token expiry cleanup task (runs every hour)
    {
        let reg = cap_registry.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
                let purged = reg.purge_expired().await;
                if purged > 0 {
                    tracing::info!("Purged {} expired capability tokens", purged);
                }
            }
        });
    }
    
    // Create app state
    // Build AuthService from security config
    let jwt_secret = if config.security.jwt_secret.is_empty() {
        eprintln!(
            "⚠️  SECURITY WARNING: jwt_secret is not configured.\n\
             \tUsing the insecure built-in default.\n\
             \tSet the JWT_SECRET environment variable or security.jwt_secret in config.toml\n\
             \tbefore deploying to production!"
        );
        "change-me-in-production-jwt-secret".into()
    } else {
        config.security.jwt_secret.clone()
    };
    let auth_service = Arc::new(crate::middleware::auth::AuthService::new(
        jwt_secret,
        config.security.jwt_expiry_hours,
        config.security.auth_enabled,
    ));

    let app_state = web::Data::new(AppState {
        task_repo: task_repo.clone(),
        config: Arc::new(config.clone()),
        plugin_manager: plugin_manager.clone(),
        auth_service: auth_service.clone(),
        event_tx: (*event_tx).clone(),
        cap_registry,
        trace_store,
        scheduler: sched,
        query_cache,
    });
    
    let server_config   = config.server.clone();
    let security_config = config.security.clone();

    // Spawn the server
    let server_handle = tokio::spawn(async move {
        tracing::info!(
            "Starting WASM-OS Server at http://{}:{}",
            server_config.host,
            server_config.port
        );
        
        let server = HttpServer::new(move || {
            // CORS: wildcard "*" is never combined with credentials — doing so
            // violates the CORS spec and would be rejected by browsers anyway.
            // For local development, allow all origins WITHOUT credentials;
            // for production, enumerate the exact allowed origins.
            let cors = if server_config.cors_origins.contains(&"*".to_string()) {
                tracing::warn!(
                    "CORS is configured to allow all origins ('*'). \
                     This is only appropriate for local development."
                );
                Cors::default()
                    .allow_any_origin()
                    .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
                    .allowed_headers(vec![
                        actix_web::http::header::AUTHORIZATION,
                        actix_web::http::header::ACCEPT,
                        actix_web::http::header::CONTENT_TYPE,
                    ])
                    .max_age(3600)
            } else {
                let mut cors = Cors::default();
                for origin in &server_config.cors_origins {
                    cors = cors.allowed_origin(origin);
                }
                cors.allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
                    .allowed_headers(vec![
                        actix_web::http::header::AUTHORIZATION,
                        actix_web::http::header::ACCEPT,
                        actix_web::http::header::CONTENT_TYPE,
                    ])
                    .supports_credentials()
                    .max_age(3600)
            };
            
            App::new()
                .wrap(TracingLogger::default())
                .wrap(RequestId)
                .wrap(SecurityHeaders)
                .wrap(RateLimiter::new(security_config.rate_limit_per_minute))
                .wrap(JwtAuth::new(app_state.auth_service.clone()))
                .wrap(cors)
                .app_data(app_state.clone())
                .app_data(
                    web::JsonConfig::default()
                        .limit(256 * 1024 * 1024) // 256 MB to support WASM binary uploads as JSON byte arrays
                        .error_handler(|err, _req| {
                            tracing::error!("JSON error: {}", err);
                            actix_web::error::InternalError::from_response(
                                err,
                                actix_web::HttpResponse::BadRequest().json(serde_json::json!({
                                    "error": "Invalid JSON",
                                    "status": 400
                                })),
                            )
                            .into()
                        }),
                )
                // Health checks
                .service(server::health_live)
                .service(server::health_live_post)
                .service(server::health_ready)
                // Metrics
                .service(server::get_metrics)
                // API v1
                .service(server::get_stats)
                .service(server::get_tasks)
                .service(server::get_task)
                .service(server::upload_task)
                .service(server::start_task)
                .service(server::stop_task)
                .service(server::delete_task)
                .service(server::update_task)
                .service(server::pause_task)
                .service(server::restart_task)
                // API v1 - Test Files
                .service(server::list_test_files)
                .service(server::run_all_test_files)
                .service(server::run_test_file)
                // API v1 - Security & Logs
                .service(server::get_task_security)
                .service(server::get_task_logs)
                // API v1 - Auth
                .service(server::get_token)
                // API v1 - Snapshots
                .service(server::list_snapshots)
                .service(server::create_snapshot)
                .service(server::delete_snapshot)
                .service(server::get_snapshot)
                // API v1 - Audit Log
                .service(server::list_audit_log)
                // API v1 - Tenants
                .service(server::list_tenants)
                .service(server::create_tenant)
                .service(server::delete_tenant)
                .service(server::get_tenant)
                // API v1 - Scheduler
                .service(server::scheduler_status)
                .service(server::scheduler_preempt)
                // API v1 - Capability Tokens
                .service(server::issue_token)
                .service(server::list_tokens)
                .service(server::revoke_token)
                .service(server::check_token)
                // API v1 - Tracing
                .service(server::list_traces)
                .service(server::get_task_traces)
                .service(server::live_trace_metrics)
                .service(server::seed_traces)
                // API v1 - Execution History
                .service(server::get_task_execution_history)
                // API v2 - Advanced Execution
                .service(advanced_execution_endpoints::execute_advanced)
                .service(advanced_execution_endpoints::execute_batch)
                .service(advanced_execution_endpoints::get_execution_report)
                .service(advanced_execution_endpoints::get_import_stats)
                .service(advanced_execution_endpoints::compare_performance)
                .service(advanced_execution_endpoints::get_advanced_metrics)
                // API v2 - Module Management & Inspection
                .service(advanced_execution_endpoints::inspect_task)
                .service(advanced_execution_endpoints::list_modules)
                .service(advanced_execution_endpoints::execute_module)
                .service(advanced_execution_endpoints::upload_module)
                // WebSocket
                .route("/ws", actix_web::web::get().to(websocket::ws_handler))
                // Static files (optional)
                // IMPORTANT: register after all explicit routes so it doesn't
                // intercept endpoints like /ws and return 404.
                .configure(configure_static_files)
        })
        .workers(server_config.workers)
        .bind((server_config.host, server_config.port))
        .expect("Failed to bind server");
        
        if let Err(e) = server.run().await {
            tracing::error!("Server error: {}", e);
        }
    });
    
    // Wait for server to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    tracing::info!("WASM-OS is ready!");
    tracing::info!("Web UI: http://{}:{}", config.server.host, config.server.port);
    tracing::info!("Health: http://{}:{}/health/live", config.server.host, config.server.port);
    tracing::info!("Metrics: http://{}:{}/metrics", config.server.host, config.server.port);
    tracing::info!("API: http://{}:{}/v1/tasks", config.server.host, config.server.port);

    // Graceful shutdown on Ctrl+C or SIGTERM
    tokio::select! {
        _ = server_handle => {
            tracing::warn!("Server task exited unexpectedly");
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Received Ctrl+C — shutting down gracefully");
        }
    }

    Ok(())
}
