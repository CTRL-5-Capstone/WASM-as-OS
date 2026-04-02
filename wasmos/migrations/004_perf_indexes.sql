-- ============================================================
-- WasmOS Schema — Migration 004
-- Performance indexes for hot-path queries identified during
-- Phase 3 profiling:
--   1. Composite (tenant_id, status) on tasks — tenant-scoped
--      status filters issued by the RBAC page and scheduler
--   2. (tenant_id, ts DESC) on audit_log — tenant-scoped audit
--      pagination, replaces full-table scans in list_audit_log()
--   3. Covering index on task_metrics including last_run_at —
--      dashboard "recent activity" sort without heap fetch
--   4. Partial index on tasks WHERE status IN ('running','pending')
--      — scheduler heartbeat + live task queries skip the
--      majority of completed/failed rows entirely
--   5. BRIN index on execution_history(started_at) — O(1)
--      metadata scan for wide time-range analytics queries;
--      complements the existing B-tree for exact lookups
-- All statements are idempotent (CREATE INDEX IF NOT EXISTS /
-- DO $$ blocks).
-- ============================================================

-- ── 1. tasks (tenant_id, status) composite ───────────────────────────────────
-- Covers: SELECT ... FROM tasks WHERE tenant_id = $1 AND status = $2
-- Used by: GET /v1/tasks?tenant_id=…&status=… and scheduler queue peek.
CREATE INDEX IF NOT EXISTS idx_tasks_tenant_status
    ON tasks(tenant_id, status)
    WHERE tenant_id IS NOT NULL;

-- ── 2. audit_log (tenant_id, ts DESC) composite ──────────────────────────────
-- Covers: SELECT ... FROM audit_log WHERE tenant_id = $1 ORDER BY ts DESC
-- Used by: GET /v1/audit?tenant_id=… (paginated audit log in RBAC page).
-- Replaces an idx_audit_tenant_id + sort-on-ts plan with a single index scan.
CREATE INDEX IF NOT EXISTS idx_audit_tenant_ts
    ON audit_log(tenant_id, ts DESC)
    WHERE tenant_id IS NOT NULL;

-- ── 3. task_metrics covering index with last_run_at ──────────────────────────
-- Covers: SELECT task_id, total_runs, last_run_at FROM task_metrics
--         ORDER BY last_run_at DESC NULLS LAST LIMIT 10
-- Used by: dashboard "recent task activity" widget.
-- INCLUDE avoids a heap fetch for the two most-read columns.
DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_indexes
        WHERE indexname = 'idx_task_metrics_last_run_covering'
    ) THEN
        CREATE INDEX idx_task_metrics_last_run_covering
            ON task_metrics(last_run_at DESC NULLS LAST)
            INCLUDE (task_id, total_runs, successful_runs, failed_runs);
    END IF;
END $$;

-- ── 4. Partial index: live tasks only (running OR pending) ───────────────────
-- Covers: SELECT ... FROM tasks WHERE status IN ('running', 'pending')
--         ORDER BY priority DESC, created_at ASC
-- Used by: scheduler work-stealing, live task monitor, dashboard running count.
-- Tiny index — only live rows, never touches historical data.
CREATE INDEX IF NOT EXISTS idx_tasks_live
    ON tasks(priority DESC, created_at ASC)
    WHERE status IN ('running', 'pending');

-- ── 5. BRIN index on execution_history(started_at) ───────────────────────────
-- Covers: time-range analytics queries like
--   SELECT AVG(duration_us), COUNT(*) FROM execution_history
--   WHERE started_at BETWEEN $1 AND $2
-- BRIN is 10-100× smaller than B-tree for append-only time-series data and
-- lets the planner skip entire block ranges outside the requested window.
-- This index coexists safely with the existing B-tree idx_exec_history_started.
DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_indexes
        WHERE indexname = 'idx_exec_history_started_brin'
    ) THEN
        CREATE INDEX idx_exec_history_started_brin
            ON execution_history USING BRIN (started_at)
            WITH (pages_per_range = 32);
    END IF;
END $$;

-- ── 6. Partial index: snapshots for a specific task, newest first ─────────────
-- Covers: SELECT ... FROM snapshots WHERE task_id = $1 ORDER BY captured_at DESC
-- Used by: GET /v1/tasks/{id}/snapshots (task detail snapshot list).
-- Avoids a separate sort on top of idx_snapshots_task.
CREATE INDEX IF NOT EXISTS idx_snapshots_task_time
    ON snapshots(task_id, captured_at DESC);
