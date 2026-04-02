pub mod models;
pub mod repository;
#[cfg(test)]
mod repository_test;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;

pub type Db = PgPool;

/// Connect to PostgreSQL and run pending migrations.
///
/// Retries the initial connection up to 5 times with exponential back-off
/// (1 s, 2 s, 4 s, 8 s) so the service survives a brief Postgres startup
/// race on container boot (common in docker-compose / k8s init containers).
///
/// Migration errors are non-fatal — logged as warnings — so the app can
/// still start when the DB user lacks DDL privileges on an already-
/// bootstrapped schema.
pub async fn connect_pg(url: &str) -> Result<Db, sqlx::Error> {
    let pool = build_pool(url).await?;
    run_migrations(&pool).await;
    Ok(pool)
}

/// Ping the database connection to verify reachability.
/// Used by the `/health` endpoint and graceful-start checks.
#[allow(dead_code)]
pub async fn health_check(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}

/// Build the connection pool with production-tuned settings.
async fn build_pool(url: &str) -> Result<Db, sqlx::Error> {
    let mut last_err = None;
    for attempt in 1..=5u32 {
        match PgPoolOptions::new()
            // Pool sizing
            .max_connections(20)
            .min_connections(2)
            // How long to wait for a free slot in the pool
            .acquire_timeout(Duration::from_secs(10))
            // Drop idle connections after 10 min — avoids stale TCP holds
            .idle_timeout(Duration::from_secs(600))
            // Recycle connections every 30 min — avoids backend-side timeouts
            .max_lifetime(Duration::from_secs(1800))
            // Skip per-checkout ping: idle_timeout + max_lifetime together
            // ensure connections are recycled before they go stale.
            // Disabling test_before_acquire removes an extra SELECT 1 on
            // every checkout, which is measurable under load.
            .test_before_acquire(false)
            .connect(url)
            .await
        {
            Ok(pool) => {
                tracing::info!(attempt, "Connected to PostgreSQL");
                return Ok(pool);
            }
            Err(e) => {
                tracing::warn!(attempt, error = %e, "PostgreSQL connection attempt failed");
                last_err = Some(e);
                if attempt < 5 {
                    // Exponential back-off: 1 s, 2 s, 4 s, 8 s
                    let backoff = Duration::from_secs(1u64 << (attempt - 1));
                    tokio::time::sleep(backoff).await;
                }
            }
        }
    }
    Err(last_err.expect("attempt loop ran at least once"))
}

/// Run pending migrations using sqlx's built-in migration runner.
///
/// Uses the `_sqlx_migrations` tracking table — safe to call on a fully
/// bootstrapped DB (becomes a no-op when all migrations are applied).
/// Unlike manual `split(";\n")` splitting, this correctly handles
/// PL/pgSQL `DO $$ … $$` blocks that contain internal semicolons, so
/// trigger and function DDL in `001_initial_schema.sql` is always applied.
async fn run_migrations(pool: &PgPool) {
    match sqlx::migrate!("./migrations").run(pool).await {
        Ok(_) => tracing::info!("PostgreSQL migrations applied"),
        Err(e) => tracing::warn!("Migration warning (non-fatal): {}", e),
    }
}
