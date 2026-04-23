use super::models::{AuditLog, ExecutionHistory, Snapshot, Task, TaskMetrics, TaskStatus, Tenant};
use super::{with_retry, Db};
use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;

pub struct TaskRepository {
    pool: Db,
}

impl TaskRepository {
    pub fn new(pool: Db) -> Self {
        Self { pool }
    }

    /// Expose pool for raw queries (e.g. migrations)
    pub fn pool(&self) -> &Db {
        &self.pool
    }

    #[allow(dead_code)]
    pub async fn create(&self, task: &Task) -> Result<(), sqlx::Error> {
        // Both INSERTs are wrapped in a single transaction so we never end up
        // with a task row that has no corresponding task_metrics row (or vice versa).
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"INSERT INTO tasks (id, name, path, status, created_at, updated_at, file_size_bytes, tenant_id, priority)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
        )
        .bind(&task.id)
        .bind(&task.name)
        .bind(&task.path)
        .bind(task.status.to_string())
        .bind(task.created_at)
        .bind(task.updated_at)
        .bind(task.file_size_bytes)
        .bind(&task.tenant_id)
        .bind(task.priority)
        .execute(&mut *tx)
        .await?;

        // Initialize a zero-metrics row — if this fails the task INSERT is rolled back.
        sqlx::query(
            r#"INSERT INTO task_metrics (task_id, total_runs, successful_runs, failed_runs, total_instructions, total_syscalls, avg_duration_us)
               VALUES ($1, 0, 0, 0, 0, 0, 0)"#,
        )
        .bind(&task.id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Option<Task>, sqlx::Error> {
        let pool = self.pool.clone();
        let id = id.to_string();
        with_retry(3, || {
            let pool = pool.clone();
            let id = id.clone();
            async move {
                sqlx::query_as::<_, Task>(
                    "SELECT id, name, path, status, created_at, updated_at, file_size_bytes, tenant_id, priority FROM tasks WHERE id = $1",
                )
                .bind(id)
                .fetch_optional(&pool)
                .await
            }
        }).await
    }

    #[allow(dead_code)]
    pub async fn get_by_name(&self, name: &str) -> Result<Option<Task>, sqlx::Error> {
        sqlx::query_as::<_, Task>(
            "SELECT id, name, path, status, created_at, updated_at, file_size_bytes, tenant_id, priority FROM tasks WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
    }

    /// Verify database reachability. Used by the `/health` endpoint.
    #[allow(dead_code)]
    pub async fn health_check(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn list_all(&self) -> Result<Vec<Task>, sqlx::Error> {
        sqlx::query_as::<_, Task>(
            "SELECT id, name, path, status, created_at, updated_at, file_size_bytes, tenant_id, priority FROM tasks ORDER BY priority DESC, created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Paginated task listing — avoids returning unbounded result sets.
    /// `limit` is clamped to 200 server-side to prevent oversized responses.
    #[allow(dead_code)]
    pub async fn list_all_paginated(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Task>, sqlx::Error> {
        let limit = limit.min(200);
        let pool = self.pool.clone();
        with_retry(3, || {
            let pool = pool.clone();
            async move {
                sqlx::query_as::<_, Task>(
                    "SELECT id, name, path, status, created_at, updated_at, file_size_bytes, tenant_id, priority \
                     FROM tasks ORDER BY priority DESC, created_at DESC LIMIT $1 OFFSET $2",
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(&pool)
                .await
            }
        }).await
    }

    /// Total task count — useful for pagination metadata in API responses.
    #[allow(dead_code)]
    pub async fn count_tasks(&self) -> Result<i64, sqlx::Error> {
        let row = sqlx::query("SELECT COUNT(*) AS n FROM tasks")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get::<i64, _>("n"))
    }

    /// Update the file path (and size) of an existing task — used when a
    /// WASM binary is re-uploaded after the task record already exists.
    #[allow(dead_code)]
    pub async fn update_task_path(
        &self,
        id: &str,
        path: &str,
        file_size_bytes: i64,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        sqlx::query(
            "UPDATE tasks SET path = $1, file_size_bytes = $2, updated_at = $3 WHERE id = $4",
        )
        .bind(path)
        .bind(file_size_bytes)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_by_status(&self, status: TaskStatus) -> Result<Vec<Task>, sqlx::Error> {
        sqlx::query_as::<_, Task>(
            "SELECT id, name, path, status, created_at, updated_at, file_size_bytes, tenant_id, priority FROM tasks WHERE status = $1 ORDER BY priority DESC, created_at DESC",
        )
        .bind(status.to_string())
        .fetch_all(&self.pool)
        .await
    }

    pub async fn list_by_tenant_and_status(
        &self,
        tenant_id: &str,
        status: TaskStatus,
    ) -> Result<Vec<Task>, sqlx::Error> {
        sqlx::query_as::<_, Task>(
            "SELECT id, name, path, status, created_at, updated_at, file_size_bytes, tenant_id, priority \
             FROM tasks WHERE tenant_id = $1 AND status = $2 ORDER BY priority DESC, created_at DESC",
        )
        .bind(tenant_id)
        .bind(status.to_string())
        .fetch_all(&self.pool)
        .await
    }

    pub async fn list_by_tenant(&self, tenant_id: &str) -> Result<Vec<Task>, sqlx::Error> {
        sqlx::query_as::<_, Task>(
            "SELECT id, name, path, status, created_at, updated_at, file_size_bytes, tenant_id, priority FROM tasks WHERE tenant_id = $1 ORDER BY priority DESC, created_at DESC",
        )
        .bind(tenant_id)
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

    /// Update task name and/or priority atomically (patch-style — only sets provided fields).
    /// A single UPDATE statement is issued regardless of which combination is supplied,
    /// so the operation is always atomic and generates exactly one WAL record.
    pub async fn update_task(
        &self,
        id: &str,
        name: Option<&str>,
        priority: Option<i16>,
    ) -> Result<(), sqlx::Error> {
        // Nothing to do — return early before touching the DB.
        if name.is_none() && priority.is_none() {
            return Ok(());
        }
        let now = Utc::now();
        match (name, priority) {
            (Some(n), Some(p)) => {
                sqlx::query(
                    "UPDATE tasks SET name = $1, priority = $2, updated_at = $3 WHERE id = $4",
                )
                .bind(n)
                .bind(p)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
            }
            (Some(n), None) => {
                sqlx::query("UPDATE tasks SET name = $1, updated_at = $2 WHERE id = $3")
                    .bind(n)
                    .bind(now)
                    .bind(id)
                    .execute(&self.pool)
                    .await?;
            }
            (None, Some(p)) => {
                sqlx::query("UPDATE tasks SET priority = $1, updated_at = $2 WHERE id = $3")
                    .bind(p)
                    .bind(now)
                    .bind(id)
                    .execute(&self.pool)
                    .await?;
            }
            (None, None) => {} // unreachable — handled above
        }
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

    /// Insert an execution record and update the task's aggregate metrics atomically.
    ///
    /// Returns the stable `execution_id` UUID that was assigned to this record.
    /// Callers can embed this in responses or WebSocket events so the client can
    /// navigate directly to `/v2/execution/{execution_id}/report`.
    pub async fn add_execution(
        &self,
        task_id: &str,
        duration_us: i64,
        success: bool,
        error: Option<String>,
        instructions: i64,
        syscalls: i64,
        memory_bytes: i64,
    ) -> Result<String, sqlx::Error> {
        let now = Utc::now();
        let execution_id = Uuid::new_v4().to_string();

        // Both writes are in one transaction: if the metrics UPDATE fails (e.g. task
        // was deleted mid-run) the history INSERT is rolled back too — no orphan rows.
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"INSERT INTO execution_history
                   (execution_id, task_id, started_at, completed_at, duration_us,
                    success, error, instructions_executed, syscalls_executed, memory_used_bytes)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"#,
        )
        .bind(&execution_id)
        .bind(task_id)
        .bind(now)
        .bind(Some(now))
        .bind(Some(duration_us))
        .bind(success)
        .bind(&error)
        .bind(instructions)
        .bind(syscalls)
        .bind(memory_bytes)
        .execute(&mut *tx)
        .await?;

        let success_inc: i64 = if success { 1 } else { 0 };
        let failed_inc: i64 = if success { 0 } else { 1 };

        sqlx::query(
            r#"UPDATE task_metrics
               SET total_runs         = total_runs + 1,
                   successful_runs    = successful_runs + $1,
                   failed_runs        = failed_runs + $2,
                   total_instructions = total_instructions + $3,
                   total_syscalls     = total_syscalls + $4,
                   avg_duration_us    = CASE
                       WHEN total_runs = 0 THEN $7
                       ELSE (avg_duration_us * total_runs + $7) / (total_runs + 1)
                   END,
                   last_run_at        = $5
               WHERE task_id = $6"#,
        )
        .bind(success_inc)
        .bind(failed_inc)
        .bind(instructions)
        .bind(syscalls)
        .bind(now)
        .bind(task_id)
        .bind(duration_us)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(execution_id)
    }

    pub async fn get_execution_history(
        &self,
        task_id: &str,
        limit: i64,
    ) -> Result<Vec<ExecutionHistory>, sqlx::Error> {
        sqlx::query_as::<_, ExecutionHistory>(
            r#"SELECT id, execution_id, task_id, started_at, completed_at, duration_us,
                      success, error, instructions_executed, syscalls_executed, memory_used_bytes
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

    /// Fetch a single execution record by its stable UUID.
    /// Used by GET /v2/execution/{execution_id}/report.
    pub async fn get_execution_by_uuid(
        &self,
        execution_id: &str,
    ) -> Result<Option<ExecutionHistory>, sqlx::Error> {
        sqlx::query_as::<_, ExecutionHistory>(
            r#"SELECT id, execution_id, task_id, started_at, completed_at, duration_us,
                      success, error, instructions_executed, syscalls_executed, memory_used_bytes
               FROM execution_history
               WHERE execution_id = $1"#,
        )
        .bind(execution_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn get_stats(&self) -> Result<SystemStats, sqlx::Error> {
        // Single atomic query — avoids 5 round-trips and race conditions between counts
        let pool = self.pool.clone();
        let row = with_retry(3, || {
            let pool = pool.clone();
            async move {
                sqlx::query(
                    r#"SELECT
                          COUNT(*)                                            AS total_tasks,
                          COUNT(*) FILTER (WHERE status = 'running')         AS running_tasks,
                          COUNT(*) FILTER (WHERE status = 'failed')          AS failed_tasks,
                          COUNT(*) FILTER (WHERE status = 'completed')       AS completed_tasks,
                          COUNT(*) FILTER (WHERE status = 'pending')         AS pending_tasks
                       FROM tasks"#,
                )
                .fetch_one(&pool)
                .await
            }
        }).await?;

        let pool2 = self.pool.clone();
        let metrics_row = with_retry(3, || {
            let pool2 = pool2.clone();
            async move {
                sqlx::query(
                    r#"SELECT
                          COALESCE(SUM(total_instructions)::BIGINT, 0) AS total_instructions,
                          COALESCE(SUM(total_syscalls)::BIGINT, 0)     AS total_syscalls,
                          COALESCE(SUM(total_runs)::BIGINT, 0)         AS total_runs,
                          COALESCE(AVG(avg_duration_us)::BIGINT, 0)    AS avg_duration_us
                       FROM task_metrics"#,
                )
                .fetch_one(&pool2)
                .await
            }
        }).await?;

        Ok(SystemStats {
            total_tasks:       row.get::<i64, _>("total_tasks") as usize,
            running_tasks:     row.get::<i64, _>("running_tasks") as usize,
            completed_tasks:   row.get::<i64, _>("completed_tasks") as usize,
            failed_tasks:      row.get::<i64, _>("failed_tasks") as usize,
            pending_tasks:     row.get::<i64, _>("pending_tasks") as usize,
            total_instructions: metrics_row.get::<i64, _>("total_instructions") as u64,
            total_syscalls:    metrics_row.get::<i64, _>("total_syscalls") as u64,
            total_runs:        metrics_row.get::<i64, _>("total_runs") as u64,
            avg_duration_us:   metrics_row.get::<i64, _>("avg_duration_us") as u64,
        })
    }

    // ─── Snapshot methods ────────────────────────────────────────────────────

    pub async fn create_snapshot(&self, snap: &Snapshot) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO snapshots (id, task_id, captured_at, state, memory_mb, instructions, stack_depth, globals_json, note)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
        )
        .bind(&snap.id)
        .bind(&snap.task_id)
        .bind(snap.captured_at)
        .bind(&snap.state)
        .bind(snap.memory_mb)
        .bind(snap.instructions)
        .bind(snap.stack_depth)
        .bind(&snap.globals_json)
        .bind(&snap.note)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_snapshots(&self, task_id: &str) -> Result<Vec<Snapshot>, sqlx::Error> {
        sqlx::query_as::<_, Snapshot>(
            "SELECT id, task_id, captured_at, state, memory_mb, instructions, stack_depth, globals_json, note FROM snapshots WHERE task_id = $1 ORDER BY captured_at DESC",
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_snapshot(&self, id: &str) -> Result<Option<Snapshot>, sqlx::Error> {
        sqlx::query_as::<_, Snapshot>(
            "SELECT id, task_id, captured_at, state, memory_mb, instructions, stack_depth, globals_json, note FROM snapshots WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn delete_snapshot(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM snapshots WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ─── Audit log methods ───────────────────────────────────────────────────

    pub async fn write_audit(
        &self,
        user_name: &str,
        role: &str,
        action: &str,
        resource: Option<&str>,
        tenant_id: Option<&str>,
        ip_addr: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO audit_log (ts, user_name, role, action, resource, tenant_id, ip_addr)
               VALUES (NOW(), $1, $2, $3, $4, $5, $6)"#,
        )
        .bind(user_name)
        .bind(role)
        .bind(action)
        .bind(resource)
        .bind(tenant_id)
        .bind(ip_addr)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_audit_log(&self, limit: i64) -> Result<Vec<AuditLog>, sqlx::Error> {
        sqlx::query_as::<_, AuditLog>(
            "SELECT id, ts, user_name, role, action, resource, tenant_id, ip_addr FROM audit_log ORDER BY ts DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Filtered audit log — action prefix match (e.g. "task." matches all task events)
    pub async fn list_audit_log_filtered(
        &self,
        action_prefix: &str,
        limit: i64,
    ) -> Result<Vec<AuditLog>, sqlx::Error> {
        let pattern = format!("{}%", action_prefix);
        sqlx::query_as::<_, AuditLog>(
            "SELECT id, ts, user_name, role, action, resource, tenant_id, ip_addr \
             FROM audit_log WHERE action LIKE $1 ORDER BY ts DESC LIMIT $2",
        )
        .bind(pattern)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    // ─── Tenant DB methods ───────────────────────────────────────────────────

    /// List all tenants ordered by creation time (newest first).
    pub async fn list_tenants(&self) -> Result<Vec<Tenant>, sqlx::Error> {
        sqlx::query_as::<_, Tenant>(
            "SELECT id, name, max_tasks, max_memory_mb, max_cpu_percent, max_concurrent, \
             max_wasm_size_mb, active, created_at \
             FROM tenants ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Fetch a single tenant by its ID — O(1) query instead of a full table scan.
    pub async fn get_tenant_by_id(&self, id: &str) -> Result<Option<Tenant>, sqlx::Error> {
        sqlx::query_as::<_, Tenant>(
            "SELECT id, name, max_tasks, max_memory_mb, max_cpu_percent, max_concurrent, \
             max_wasm_size_mb, active, created_at \
             FROM tenants WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Count how many tasks are currently owned by a tenant.
    /// Used to enforce the `max_tasks` quota at upload time.
    pub async fn count_tasks_by_tenant(&self, tenant_id: &str) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM tasks WHERE tenant_id = $1",
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn create_tenant(
        &self,
        id: &str,
        name: &str,
        max_tasks: i32,
        max_memory_mb: i32,
        max_cpu_percent: i16,
        max_concurrent: i32,
        max_wasm_size_mb: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO tenants (id, name, max_tasks, max_memory_mb, max_cpu_percent, max_concurrent, max_wasm_size_mb)
               VALUES ($1, $2, $3, $4, $5, $6, $7)"#
        )
        .bind(id)
        .bind(name)
        .bind(max_tasks)
        .bind(max_memory_mb)
        .bind(max_cpu_percent)
        .bind(max_concurrent)
        .bind(max_wasm_size_mb)
        .execute(&self.pool)
        .await?;  // unique constraint violation propagates as sqlx::Error → 409 Conflict
        Ok(())
    }

    pub async fn delete_tenant(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM tenants WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[derive(Debug, serde::Serialize)]
pub struct SystemStats {
    pub total_tasks: usize,
    pub running_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub pending_tasks: usize,
    pub total_instructions: u64,
    pub total_syscalls: u64,
    pub total_runs: u64,
    pub avg_duration_us: u64,
}
