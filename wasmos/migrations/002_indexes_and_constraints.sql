-- ============================================================
-- WasmOS Schema — Additive improvements
-- Migration: 002_indexes_and_constraints.sql
-- Adds missing indexes and referential integrity guards.
-- All statements are idempotent (IF NOT EXISTS / DO $$ blocks).
-- ============================================================

-- ── Partial index: fast "failed runs" queries ────────────────────────────────
-- Covers: SELECT ... FROM execution_history WHERE task_id = $1 AND success = false
-- Previously required a full scan of the task's history.
CREATE INDEX IF NOT EXISTS idx_exec_history_failures
    ON execution_history(task_id, started_at DESC)
    WHERE success = FALSE;

-- ── audit_log.tenant_id → tenants(id) soft FK ────────────────────────────────
-- When a tenant is deleted, existing audit entries keep their tenant_id for
-- forensic traceability but it is set to NULL so joins don't break.
DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'fk_audit_log_tenant_id'
    ) THEN
        ALTER TABLE audit_log
            ADD CONSTRAINT fk_audit_log_tenant_id
            FOREIGN KEY (tenant_id)
            REFERENCES tenants(id)
            ON DELETE SET NULL
            DEFERRABLE INITIALLY DEFERRED;
    END IF;
END $$;

-- ── Index on audit_log.tenant_id for tenant-scoped log queries ───────────────
CREATE INDEX IF NOT EXISTS idx_audit_tenant_id ON audit_log(tenant_id)
    WHERE tenant_id IS NOT NULL;

-- ── Index on execution_history: system-wide performance analytics ────────────
-- Covers: SELECT AVG(duration_us), COUNT(*) ... GROUP BY success (dashboard)
CREATE INDEX IF NOT EXISTS idx_exec_history_perf
    ON execution_history(started_at DESC, success, duration_us)
    WHERE duration_us IS NOT NULL;
