-- ============================================================
-- WasmOS Schema — Migration 003
-- Adds:
--   1. execution_id UUID column to execution_history (links dispatcher
--      UUIDs to DB rows so /v2/execution/{id}/report works reliably)
--   2. idx_tenants_name index for fast name lookups (UNIQUE constraint
--      already enforces uniqueness but doesn't create a usable B-tree)
-- All statements are idempotent (IF NOT EXISTS / DO $$ blocks).
-- ============================================================

-- ── 1. Add execution_id to execution_history ─────────────────────────────────
-- Step A: Add nullable first so the backfill doesn't violate NOT NULL
DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'execution_history' AND column_name = 'execution_id'
    ) THEN
        ALTER TABLE execution_history
            ADD COLUMN execution_id TEXT DEFAULT NULL;
    END IF;
END $$;

-- Step B: Backfill any existing rows that don't have an execution_id yet
UPDATE execution_history
SET    execution_id = gen_random_uuid()::TEXT
WHERE  execution_id IS NULL;

-- Step C: Enforce NOT NULL going forward
DO $$ BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'execution_history'
          AND column_name = 'execution_id'
          AND is_nullable = 'YES'
    ) THEN
        ALTER TABLE execution_history
            ALTER COLUMN execution_id SET NOT NULL,
            ALTER COLUMN execution_id SET DEFAULT gen_random_uuid()::TEXT;
    END IF;
END $$;

-- Step D: Unique constraint (idempotent)
DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'uq_exec_history_execution_id'
    ) THEN
        ALTER TABLE execution_history
            ADD CONSTRAINT uq_exec_history_execution_id UNIQUE (execution_id);
    END IF;
END $$;

-- Step E: Index for fast UUID-based lookups from /v2/execution/{id}/report
CREATE INDEX IF NOT EXISTS idx_exec_history_execution_id
    ON execution_history(execution_id);

-- ── 2. Index on tenants.name ─────────────────────────────────────────────────
-- UNIQUE constraint already existed but PostgreSQL creates the B-tree
-- index automatically for UNIQUE. This is an explicit named index for
-- clarity and to ensure it is listed in pg_indexes.
CREATE INDEX IF NOT EXISTS idx_tenants_name ON tenants(name);
