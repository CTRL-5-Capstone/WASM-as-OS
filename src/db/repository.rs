use super::models::{ExecutionHistory, Task, TaskMetrics, TaskStatus};
use super::Db;
use chrono::Utc;
use sqlx::Row;

pub struct TaskRepository {
    pool: Db,
}

impl TaskRepository {
    pub fn new(pool: Db) -> Self {
        Self { pool }
    }

    pub async fn create(&self, task: &Task) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO tasks (id, name, path, status, created_at, updated_at, file_size_bytes)
               VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
        )
        .bind(&task.id)
        .bind(&task.name)
        .bind(&task.path)
        .bind(task.status.to_string())
        .bind(task.created_at)
        .bind(task.updated_at)
        .bind(task.file_size_bytes)
        .execute(&self.pool)
        .await?;

        // Initialize metrics
        sqlx::query(
            r#"INSERT INTO task_metrics (task_id, total_runs, successful_runs, failed_runs, total_instructions, total_syscalls, avg_duration_us)
               VALUES ($1, 0, 0, 0, 0, 0, 0)"#,
        )
        .bind(&task.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Option<Task>, sqlx::Error> {
        sqlx::query_as::<_, Task>(
            "SELECT id, name, path, status, created_at, updated_at, file_size_bytes FROM tasks WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn get_by_name(&self, name: &str) -> Result<Option<Task>, sqlx::Error> {
        sqlx::query_as::<_, Task>(
            "SELECT id, name, path, status, created_at, updated_at, file_size_bytes FROM tasks WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_all(&self) -> Result<Vec<Task>, sqlx::Error> {
        sqlx::query_as::<_, Task>(
            "SELECT id, name, path, status, created_at, updated_at, file_size_bytes FROM tasks ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn list_by_status(&self, status: TaskStatus) -> Result<Vec<Task>, sqlx::Error> {
        sqlx::query_as::<_, Task>(
            "SELECT id, name, path, status, created_at, updated_at, file_size_bytes FROM tasks WHERE status = $1 ORDER BY created_at DESC",
        )
        .bind(status.to_string())
        .fetch_all(&self.pool)
        .await
    }

    pub async fn update_status(&self, id: &str, status: TaskStatus) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        sqlx::query("UPDATE tasks SET status = $1, updated_at = $2 WHERE id = $3")
            .bind(status.to_string())
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete(&self, id: &str) -> Result<(), sqlx::Error> {
        // CASCADE will handle task_metrics and execution_history
        sqlx::query("DELETE FROM tasks WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_metrics(&self, task_id: &str) -> Result<Option<TaskMetrics>, sqlx::Error> {
        sqlx::query_as::<_, TaskMetrics>(
            "SELECT task_id, total_runs, successful_runs, failed_runs, total_instructions, total_syscalls, avg_duration_us, last_run_at FROM task_metrics WHERE task_id = $1",
        )
        .bind(task_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn add_execution(
        &self,
        task_id: &str,
        duration_us: i64,
        success: bool,
        error: Option<String>,
        instructions: i64,
        syscalls: i64,
        memory_bytes: i64,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now();

        // Insert execution history
        sqlx::query(
            r#"INSERT INTO execution_history (task_id, started_at, completed_at, duration_us, success, error, instructions_executed, syscalls_executed, memory_used_bytes)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
        )
        .bind(task_id)
        .bind(now)
        .bind(Some(now))
        .bind(Some(duration_us))
        .bind(success)
        .bind(&error)
        .bind(instructions)
        .bind(syscalls)
        .bind(memory_bytes)
        .execute(&self.pool)
        .await?;

        // Update metrics
        let success_inc: i64 = if success { 1 } else { 0 };
        let failed_inc: i64 = if success { 0 } else { 1 };

        sqlx::query(
            r#"UPDATE task_metrics
               SET total_runs = total_runs + 1,
                   successful_runs = successful_runs + $1,
                   failed_runs = failed_runs + $2,
                   total_instructions = total_instructions + $3,
                   total_syscalls = total_syscalls + $4,
                   last_run_at = $5
               WHERE task_id = $6"#,
        )
        .bind(success_inc)
        .bind(failed_inc)
        .bind(instructions)
        .bind(syscalls)
        .bind(now)
        .bind(task_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_execution_history(
        &self,
        task_id: &str,
        limit: i64,
    ) -> Result<Vec<ExecutionHistory>, sqlx::Error> {
        sqlx::query_as::<_, ExecutionHistory>(
            r#"SELECT task_id, started_at, completed_at, duration_us, success, error, instructions_executed, syscalls_executed, memory_used_bytes
               FROM execution_history
               WHERE task_id = $1
               ORDER BY started_at DESC
               LIMIT $2"#,
        )
        .bind(task_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_stats(&self) -> Result<SystemStats, sqlx::Error> {
        let total_tasks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&self.pool)
            .await?;

        let running_tasks: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE status = 'running'")
                .fetch_one(&self.pool)
                .await?;

        let failed_tasks: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE status = 'failed'")
                .fetch_one(&self.pool)
                .await?;

        let row = sqlx::query(
            r#"SELECT COALESCE(SUM(total_instructions)::BIGINT, 0) as total_instructions,
                      COALESCE(SUM(total_syscalls)::BIGINT, 0) as total_syscalls
               FROM task_metrics"#,
        )
        .fetch_one(&self.pool)
        .await?;

        let total_instructions: i64 = row.get("total_instructions");
        let total_syscalls: i64 = row.get("total_syscalls");

        Ok(SystemStats {
            total_tasks: total_tasks as usize,
            running_tasks: running_tasks as usize,
            failed_tasks: failed_tasks as usize,
            total_instructions: total_instructions as u64,
            total_syscalls: total_syscalls as u64,
        })
    }
}

#[derive(Debug, serde::Serialize)]
pub struct SystemStats {
    pub total_tasks: usize,
    pub running_tasks: usize,
    pub failed_tasks: usize,
    pub total_instructions: u64,
    pub total_syscalls: u64,
}
