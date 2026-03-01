pub mod models;
pub mod repository;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub type Db = PgPool;

pub async fn connect_pg(url: &str) -> Result<Db, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(url)
        .await?;

    tracing::info!("Connected to PostgreSQL at {}", url);

    // Create tables if they don't exist
    create_schema(&pool).await?;

    Ok(pool)
}

async fn create_schema(pool: &PgPool) -> Result<(), sqlx::Error> {
    // Each statement must be executed separately — PG doesn't allow
    // multiple statements in a single prepared statement.

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            path TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            file_size_bytes BIGINT NOT NULL DEFAULT 0
        )"#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS task_metrics (
            task_id TEXT PRIMARY KEY REFERENCES tasks(id) ON DELETE CASCADE,
            total_runs BIGINT NOT NULL DEFAULT 0,
            successful_runs BIGINT NOT NULL DEFAULT 0,
            failed_runs BIGINT NOT NULL DEFAULT 0,
            total_instructions BIGINT NOT NULL DEFAULT 0,
            total_syscalls BIGINT NOT NULL DEFAULT 0,
            avg_duration_us BIGINT NOT NULL DEFAULT 0,
            last_run_at TIMESTAMPTZ
        )"#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS execution_history (
            id SERIAL PRIMARY KEY,
            task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            completed_at TIMESTAMPTZ,
            duration_us BIGINT,
            success BOOLEAN NOT NULL DEFAULT FALSE,
            error TEXT,
            instructions_executed BIGINT NOT NULL DEFAULT 0,
            syscalls_executed BIGINT NOT NULL DEFAULT 0,
            memory_used_bytes BIGINT NOT NULL DEFAULT 0
        )"#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_name ON tasks(name)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_execution_history_task_id ON execution_history(task_id, started_at DESC)")
        .execute(pool)
        .await?;

    tracing::info!("PostgreSQL schema ensured");
    Ok(())
}
