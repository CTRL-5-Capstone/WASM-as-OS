-- ============================================================
-- WasmOS Production Schema
-- Migration: 001_initial_schema.sql
-- Single source of truth — authoritative over mod.rs create_schema().
-- All statements are idempotent (IF NOT EXISTS / CREATE OR REPLACE).
-- ============================================================

-- ── auto-update trigger function ────────────────────────────────────────────
-- Sets updated_at = NOW() before every UPDATE that touches a row.
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;

-- ── tasks ────────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS tasks (
    id              TEXT        PRIMARY KEY,
    name            TEXT        NOT NULL,
    path            TEXT        NOT NULL,
    status          TEXT        NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending','running','completed','failed','stopped')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    file_size_bytes BIGINT      NOT NULL DEFAULT 0 CHECK (file_size_bytes >= 0),
    tenant_id       TEXT        DEFAULT NULL,
    priority        SMALLINT    NOT NULL DEFAULT 5 CHECK (priority BETWEEN 1 AND 10)
);

-- Trigger: keep updated_at current on every row update
DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger
        WHERE tgname = 'tasks_set_updated_at'
          AND tgrelid = 'tasks'::regclass
    ) THEN
        CREATE TRIGGER tasks_set_updated_at
        BEFORE UPDATE ON tasks
        FOR EACH ROW EXECUTE FUNCTION set_updated_at();
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_tasks_status          ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_tenant_id       ON tasks(tenant_id);
CREATE INDEX IF NOT EXISTS idx_tasks_created_at      ON tasks(created_at DESC);
-- Composite: scheduler picks up pending tasks ordered by priority + age
CREATE INDEX IF NOT EXISTS idx_tasks_sched           ON tasks(status, priority DESC, created_at ASC)
    WHERE status = 'pending';

-- ── task_metrics ─────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS task_metrics (
    task_id             TEXT        PRIMARY KEY REFERENCES tasks(id) ON DELETE CASCADE,
    total_runs          BIGINT      NOT NULL DEFAULT 0 CHECK (total_runs >= 0),
    successful_runs     BIGINT      NOT NULL DEFAULT 0 CHECK (successful_runs >= 0),
    failed_runs         BIGINT      NOT NULL DEFAULT 0 CHECK (failed_runs >= 0),
    total_instructions  BIGINT      NOT NULL DEFAULT 0 CHECK (total_instructions >= 0),
    total_syscalls      BIGINT      NOT NULL DEFAULT 0 CHECK (total_syscalls >= 0),
    avg_duration_us     BIGINT      NOT NULL DEFAULT 0 CHECK (avg_duration_us >= 0),
    last_run_at         TIMESTAMPTZ DEFAULT NULL
);

-- ── execution_history ────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS execution_history (
    id                    BIGSERIAL   PRIMARY KEY,
    task_id               TEXT        NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    started_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at          TIMESTAMPTZ DEFAULT NULL,
    duration_us           BIGINT      DEFAULT NULL CHECK (duration_us IS NULL OR duration_us >= 0),
    -- FALSE until updated by add_execution(); always set before commit
    success               BOOLEAN     NOT NULL DEFAULT FALSE,
    error                 TEXT        DEFAULT NULL,
    instructions_executed BIGINT      NOT NULL DEFAULT 0 CHECK (instructions_executed >= 0),
    syscalls_executed     BIGINT      NOT NULL DEFAULT 0 CHECK (syscalls_executed >= 0),
    memory_used_bytes     BIGINT      NOT NULL DEFAULT 0 CHECK (memory_used_bytes >= 0)
);

-- Most common query: latest N runs for one task
CREATE INDEX IF NOT EXISTS idx_exec_history_task_time ON execution_history(task_id, started_at DESC);
-- System-wide chronological scan (analytics, monitoring)
CREATE INDEX IF NOT EXISTS idx_exec_history_started   ON execution_history(started_at DESC);
-- Quick success-rate queries
CREATE INDEX IF NOT EXISTS idx_exec_history_success   ON execution_history(task_id, success);

-- ── tenants ──────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS tenants (
    id               TEXT        PRIMARY KEY,
    name             TEXT        NOT NULL UNIQUE,
    max_tasks        INTEGER     NOT NULL DEFAULT 100  CHECK (max_tasks > 0),
    max_memory_mb    INTEGER     NOT NULL DEFAULT 512  CHECK (max_memory_mb > 0),
    max_cpu_percent  SMALLINT    NOT NULL DEFAULT 80   CHECK (max_cpu_percent BETWEEN 1 AND 100),
    max_concurrent   INTEGER     NOT NULL DEFAULT 10   CHECK (max_concurrent > 0),
    max_wasm_size_mb INTEGER     NOT NULL DEFAULT 50   CHECK (max_wasm_size_mb > 0),
    active           BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ── audit_log ────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS audit_log (
    id          BIGSERIAL   PRIMARY KEY,
    ts          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    user_name   TEXT        NOT NULL DEFAULT 'system',
    role        TEXT        NOT NULL DEFAULT 'system',
    action      TEXT        NOT NULL,
    resource    TEXT        DEFAULT NULL,
    tenant_id   TEXT        DEFAULT NULL,
    ip_addr     TEXT        DEFAULT NULL
);

CREATE INDEX IF NOT EXISTS idx_audit_ts     ON audit_log(ts DESC);
CREATE INDEX IF NOT EXISTS idx_audit_user   ON audit_log(user_name);
-- Filtered audit queries (e.g. action LIKE 'task.%')
CREATE INDEX IF NOT EXISTS idx_audit_action ON audit_log(action);

-- ── snapshots ────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS snapshots (
    id              TEXT        PRIMARY KEY,
    task_id         TEXT        NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    captured_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    state           TEXT        NOT NULL DEFAULT 'idle',
    memory_mb       REAL        NOT NULL DEFAULT 0 CHECK (memory_mb >= 0),
    instructions    BIGINT      NOT NULL DEFAULT 0 CHECK (instructions >= 0),
    stack_depth     INTEGER     NOT NULL DEFAULT 0 CHECK (stack_depth >= 0),
    globals_json    TEXT        NOT NULL DEFAULT '{}',
    note            TEXT        DEFAULT NULL
);

CREATE INDEX IF NOT EXISTS idx_snapshots_task        ON snapshots(task_id);
CREATE INDEX IF NOT EXISTS idx_snapshots_captured_at ON snapshots(captured_at DESC);
