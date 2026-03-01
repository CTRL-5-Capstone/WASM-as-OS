// Tests require a running PostgreSQL instance.
// Set WASMOS__DATABASE__URL=postgresql://postgres:postgres@localhost:5432/wasmos_test
// and create the database before running these tests.
//
// To run: cargo test -- --ignored

#[cfg(test)]
mod tests {
    // Integration tests are disabled by default since they require a running PostgreSQL instance.
    // Uncomment and update when a test database is available.

    /*
    use actix_web::{test, web, App};
    use serde_json::json;

    use crate::{
        config::Config,
        db::{connect_pg, repository::TaskRepository},
        server::*,
    };

    async fn setup_test_app() -> (
        impl actix_web::dev::Service<
            actix_web::dev::ServiceRequest,
            Response = actix_web::dev::ServiceResponse,
            Error = actix_web::Error,
        >,
        TaskRepository,
    ) {
        let pool = connect_pg("postgresql://postgres:postgres@localhost:5432/wasmos_test")
            .await
            .expect("Failed to connect to test database");

        let task_repo = TaskRepository::new(pool);
        let config = Config::load().unwrap_or_else(|_| {
            Config {
                server: crate::config::ServerConfig {
                    host: "127.0.0.1".parse().unwrap(),
                    port: 8080,
                    workers: 1,
                    cors_origins: vec!["*".to_string()],
                },
                database: crate::config::DatabaseConfig {
                    url: "postgresql://postgres:postgres@localhost:5432/wasmos_test".to_string(),
                },
                security: crate::config::SecurityConfig {
                    auth_enabled: false,
                    jwt_secret: "test_secret".to_string(),
                    jwt_expiry_hours: 24,
                    rate_limit_per_minute: 60,
                },
                limits: crate::config::ResourceLimits {
                    max_wasm_size_mb: 50,
                    max_memory_mb: 128,
                    execution_timeout_secs: 30,
                    max_instructions: 10_000_000,
                    max_stack_depth: 1024,
                    max_concurrent_tasks: 100,
                },
                logging: crate::config::LoggingConfig {
                    level: "info".to_string(),
                    format: "pretty".to_string(),
                },
            }
        });

        let app_state = web::Data::new(AppState {
            task_repo: std::sync::Arc::new(task_repo.clone()),
            config: std::sync::Arc::new(config),
        });

        let app = test::init_service(
            App::new()
                .app_data(app_state)
                .service(health_live)
                .service(health_ready)
                .service(get_stats)
                .service(get_tasks)
                .service(get_task)
                .service(upload_task)
                .service(start_task)
                .service(stop_task)
                .service(delete_task)
        ).await;

        (app, task_repo)
    }

    #[ignore]
    #[actix_web::test]
    async fn test_health_live() {
        let (app, _) = setup_test_app().await;
        let req = test::TestRequest::get().uri("/health/live").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[ignore]
    #[actix_web::test]
    async fn test_get_tasks_empty() {
        let (app, _) = setup_test_app().await;
        let req = test::TestRequest::get().uri("/v1/tasks").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
    */
}
