mod config;
mod db;
mod error;
mod metrics;
mod middleware;
mod run_wasm;
mod server;
mod telemetry;

use crate::config::Config;
use crate::db::{connect_pg, repository::TaskRepository};
use crate::middleware::{logging::RequestId, rate_limit::RateLimiter};
use crate::server::AppState;
use actix_cors::Cors;
use actix_files::Files;
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
    
    // Connect to PostgreSQL
    let pool = connect_pg(&config.database.url)
        .await
        .expect("Failed to connect to PostgreSQL");
    
    // Create repository
    let task_repo = Arc::new(TaskRepository::new(pool));
    
    // Create app state
    let app_state = web::Data::new(AppState {
        task_repo: task_repo.clone(),
        config: Arc::new(config.clone()),
    });
    
    let server_config = config.server.clone();
    let security_config = config.security.clone();
    
    // Spawn the server
    let server_handle = tokio::spawn(async move {
        tracing::info!(
            "Starting WASM-OS Server at http://{}:{}",
            server_config.host,
            server_config.port
        );
        
        let server = HttpServer::new(move || {
            let cors = if server_config.cors_origins.contains(&"*".to_string()) {
                Cors::permissive()
            } else {
                let mut cors = Cors::default();
                for origin in &server_config.cors_origins {
                    cors = cors.allowed_origin(origin);
                }
                cors.allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                    .allowed_headers(vec![
                        actix_web::http::header::AUTHORIZATION,
                        actix_web::http::header::ACCEPT,
                        actix_web::http::header::CONTENT_TYPE,
                    ])
            };
            
            App::new()
                .wrap(TracingLogger::default())
                .wrap(cors)
                .wrap(RequestId)
                .wrap(RateLimiter::new(security_config.rate_limit_per_minute))
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
                // Static files
                .service(Files::new("/", "./web").index_file("index.html"))
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
    
    // Wait for server to complete
    server_handle.await.unwrap();
    
    Ok(())
}
