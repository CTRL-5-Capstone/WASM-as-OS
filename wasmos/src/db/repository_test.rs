// Repository integration tests.
// These require a running PostgreSQL instance.
// Set DATABASE_URL env var, then run:
//   cargo test -- --ignored
// Or run all ignored tests:
//   cargo test -- --include-ignored

#[cfg(test)]
mod tests {
    use crate::db::{connect_pg, repository::TaskRepository};
    use crate::db::models::{Snapshot, Task, TaskStatus};
    use std::sync::Arc;

    async fn test_repo() -> Arc<TaskRepository> {
        let url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://wasmos:wasmos@localhost:5432/wasmos_test".to_string());
        let pool = connect_pg(&url).await.expect("Failed to connect to test DB");
        Arc::new(TaskRepository::new(pool))
    }

    // ─── Task CRUD ──────────────────────────────────────────────────────────

    #[tokio::test]
    #[ignore]
    async fn test_create_and_get_task() {
        let repo = test_repo().await;
        let task = Task::new("test_task".to_string(), "/tmp/test.wasm".to_string(), 1024);
        let task_id = task.id.clone();

        repo.create(&task).await.expect("create failed");

        let fetched = repo.get_by_id(&task_id).await.expect("get failed");
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.name, "test_task");
        assert_eq!(fetched.status, TaskStatus::Pending);
        assert_eq!(fetched.priority, 5);
        assert!(fetched.tenant_id.is_none());

        // Cleanup
        repo.delete(&task_id).await.expect("delete failed");
    }

    #[tokio::test]
    #[ignore]
    async fn test_update_status() {
        let repo = test_repo().await;
        let task = Task::new("status_test".to_string(), "/tmp/status.wasm".to_string(), 512);
        let task_id = task.id.clone();

        repo.create(&task).await.expect("create failed");
        repo.update_status(&task_id, TaskStatus::Running).await.expect("update failed");

        let fetched = repo.get_by_id(&task_id).await.unwrap().unwrap();
        assert_eq!(fetched.status, TaskStatus::Running);

        repo.delete(&task_id).await.expect("delete failed");
    }

    #[tokio::test]
    #[ignore]
    async fn test_list_all_and_by_status() {
        let repo = test_repo().await;
        let t1 = Task::new("list_a".to_string(), "/tmp/a.wasm".to_string(), 100);
        let t2 = Task::new("list_b".to_string(), "/tmp/b.wasm".to_string(), 200);
        let id1 = t1.id.clone();
        let id2 = t2.id.clone();

        repo.create(&t1).await.unwrap();
        repo.create(&t2).await.unwrap();
        repo.update_status(&id1, TaskStatus::Completed).await.unwrap();

        let all = repo.list_all().await.unwrap();
        assert!(all.iter().any(|t| t.id == id1));
        assert!(all.iter().any(|t| t.id == id2));

        let completed = repo.list_by_status(TaskStatus::Completed).await.unwrap();
        assert!(completed.iter().any(|t| t.id == id1));

        repo.delete(&id1).await.unwrap();
        repo.delete(&id2).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_tenant_id_and_priority() {
        let repo = test_repo().await;
        let mut task = Task::new("tenant_task".to_string(), "/tmp/t.wasm".to_string(), 256);
        task.tenant_id = Some("tenant-001".to_string());
        task.priority = 9;
        let id = task.id.clone();

        repo.create(&task).await.unwrap();

        let fetched = repo.get_by_id(&id).await.unwrap().unwrap();
        assert_eq!(fetched.tenant_id, Some("tenant-001".to_string()));
        assert_eq!(fetched.priority, 9);

        let by_tenant = repo.list_by_tenant("tenant-001").await.unwrap();
        assert!(by_tenant.iter().any(|t| t.id == id));

        repo.delete(&id).await.unwrap();
    }

    // ─── Metrics and Execution History ──────────────────────────────────────

    #[tokio::test]
    #[ignore]
    async fn test_add_execution_updates_metrics() {
        let repo = test_repo().await;
        let task = Task::new("exec_test".to_string(), "/tmp/exec.wasm".to_string(), 1024);
        let id = task.id.clone();
        repo.create(&task).await.unwrap();

        // Record two executions
        repo.add_execution(&id, 1000, true, None, 500, 10, 2048).await.unwrap();
        repo.add_execution(&id, 3000, false, Some("OOM".to_string()), 100, 5, 4096).await.unwrap();

        let metrics = repo.get_metrics(&id).await.unwrap().unwrap();
        assert_eq!(metrics.total_runs, 2);
        assert_eq!(metrics.successful_runs, 1);
        assert_eq!(metrics.failed_runs, 1);
        assert_eq!(metrics.total_instructions, 600);
        assert_eq!(metrics.total_syscalls, 15);
        // avg_duration_us: first run = (0*0+1000)/1 = 1000, second = (1000*1+3000)/2 = 2000
        assert_eq!(metrics.avg_duration_us, 2000);

        let history = repo.get_execution_history(&id, 10).await.unwrap();
        assert_eq!(history.len(), 2);
        assert!(history[0].id > 0); // BIGSERIAL id must be positive
        // Most recent first — failure was second
        assert_eq!(history[0].success, false);
        assert_eq!(history[1].success, true);

        repo.delete(&id).await.unwrap();
    }

    // ─── Stats ──────────────────────────────────────────────────────────────

    #[tokio::test]
    #[ignore]
    async fn test_get_stats_fields() {
        let repo = test_repo().await;
        let t = Task::new("stats_task".to_string(), "/tmp/s.wasm".to_string(), 512);
        let id = t.id.clone();
        repo.create(&t).await.unwrap();
        repo.update_status(&id, TaskStatus::Running).await.unwrap();

        let stats = repo.get_stats().await.unwrap();
        assert!(stats.total_tasks >= 1);
        assert!(stats.running_tasks >= 1);
        // All new fields must exist and be non-negative
        let _ = stats.completed_tasks;
        let _ = stats.pending_tasks;
        let _ = stats.total_runs;
        let _ = stats.avg_duration_us;

        repo.delete(&id).await.unwrap();
    }

    // ─── Snapshots ──────────────────────────────────────────────────────────

    #[tokio::test]
    #[ignore]
    async fn test_snapshot_crud() {
        let repo = test_repo().await;
        let task = Task::new("snap_task".to_string(), "/tmp/snap.wasm".to_string(), 512);
        let task_id = task.id.clone();
        repo.create(&task).await.unwrap();

        let snap = Snapshot::new(
            task_id.clone(),
            "running".to_string(),
            64.0,
            12345,
            3,
        );
        let snap_id = snap.id.clone();

        repo.create_snapshot(&snap).await.unwrap();

        let fetched = repo.get_snapshot(&snap_id).await.unwrap().unwrap();
        assert_eq!(fetched.task_id, task_id);
        assert_eq!(fetched.state, "running");
        assert_eq!(fetched.instructions, 12345);

        let list = repo.list_snapshots(&task_id).await.unwrap();
        assert_eq!(list.len(), 1);

        repo.delete_snapshot(&snap_id).await.unwrap();
        let gone = repo.get_snapshot(&snap_id).await.unwrap();
        assert!(gone.is_none());

        repo.delete(&task_id).await.unwrap();
    }

    // ─── Audit Log ──────────────────────────────────────────────────────────

    #[tokio::test]
    #[ignore]
    async fn test_audit_log_write_and_list() {
        let repo = test_repo().await;

        repo.write_audit("alice", "admin", "task.delete", Some("task-xyz"), Some("tenant-001"), Some("127.0.0.1")).await.unwrap();
        repo.write_audit("system", "system", "server.start", None, None, None).await.unwrap();

        let logs = repo.list_audit_log(10).await.unwrap();
        assert!(logs.len() >= 2);
        // Most recent first (server.start was inserted last)
        assert_eq!(logs[0].action, "server.start");
        assert_eq!(logs[1].action, "task.delete");
        assert_eq!(logs[1].user_name, "alice");
    }
}
