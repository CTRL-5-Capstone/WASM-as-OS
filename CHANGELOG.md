# Changelog

We try to keep this up to date as things change. The format is loosely based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## Unreleased (development branch)

This section covers everything that's been merged since the last tagged release.

### Infrastructure changes

We **dropped Docker entirely** — no more Dockerfile, docker-compose files, .dockerignore, or the nginx directory. Everything builds with Nixpacks now, which is what Railway, Render, and Fly.io all use under the hood. This simplified the deployment story quite a bit.

New files that came out of this:

- `nixpacks.toml` for the build configuration
- `Procfile` and `Procfile.dev` for process management
- `railway.json` (both root and `frontend/`) for Railway's service config
- `DEPLOYMENT.md` covering all three cloud platforms with environment variable references and CORS setup

### Backend improvements

**Error handling** got a proper overhaul. We added `Unauthorized` (401) and `NotFound` (404) variants to `WasmOsError`, and every error JSON response now includes a machine-readable `code` field (`NOT_FOUND`, `VALIDATION_ERROR`, etc.). Raw database error messages no longer leak to HTTP clients.

**Security hardening:**
- The server logs a `[SECURITY WARNING]` at startup if you're running with insecure defaults for `admin_key`, `jwt_secret`, or CORS
- `POST /v1/auth/token` returns 401 on a bad key (used to be 400) and doesn't hint whether the key was missing or wrong
- Path traversal protection on task file reads — resolved paths are canonicalized and checked against the `wasm_files/` sandbox directory
- Per-file 30-second timeout in `POST /v1/test-files/run-all` so a stuck WASM module can't block the entire batch

**Performance:**
- `update_task` in the repository now issues a single atomic UPDATE (was two sequential queries)
- Prometheus path-label normalization in the logging middleware — UUID and numeric path segments get replaced with `{id}` to prevent cardinality explosion
- Added performance indexes (migration 004): composite `(tenant_id, status)` on tasks, covering index on task_metrics with `last_run_at`, partial index for live tasks only, BRIN index on execution_history for time-range analytics

**Database:**
- `create_schema()` was removed from `db/mod.rs`; `connect_pg()` now applies `001_initial_schema.sql` as the single source of truth
- Duplicate migration call removed from `main.rs`
- `execution_history.success` column fixed: migration now correctly uses `NOT NULL DEFAULT FALSE` (was `DEFAULT NULL`)
- Added `execution_id` UUID column to execution_history (migration 003) for stable `/v2/execution/{id}/report` links
- Several missing indexes added: `idx_tasks_sched`, `idx_exec_history_success`, `idx_audit_action`, `idx_snapshots_captured_at`

### Frontend — dashboard pages

The Next.js dashboard has grown significantly. Here's a rundown of what's been added and improved:

**Tasks page (v5.0+):** Beyond the basics of uploading and running WASM files, the tasks page now includes a full suite of environment simulation tools. There's an ABI mocking dashboard where you can define mock sensor inputs, a virtual filesystem explorer for managing files the WASM module can access, a runtime environment variables editor, and a scenario orchestrator for running chaos-engineering-style tests with sequences of injected events.

**Traces page (v3.0):** The tracing view went from a basic list to a full environment-aware observability tool. It now generates synthetic spans for ABI sensor reads and vFS I/O operations, displays scenario result badges ("Assertion Passed" / "Policy Violation" instead of generic OK/FAIL), includes environment-aware heatmaps with sensor and vFS tooltips, auto-captures forensic snapshots on violations, and has a "Clone to Test" feature for saving trace + environment as a regression test case.

**Snapshots page (v3.0):** Snapshots used to just show memory/instructions/stack data. Now each snapshot carries an environment "sidecar" (the mock sensor state, vFS files, and environment variables that were active at capture time). You can diff two snapshots side-by-side including their environment state, fork a snapshot with modified inputs, and see a visual mapping of which memory regions were loaded from which virtual files. Forensic snapshots (auto-captured on trace violations) get special tagging and can be linked back to the originating trace.

**Security page:** Static binary analysis with control-flow graph visualization, entropy heatmaps, import/export inspection, and YARA-style pattern matching for suspicious byte sequences.

**Other pages:** Audit log with playback, RBAC/capability token management, batch execution, real-time monitoring, analytics dashboards, and an xterm.js terminal with WebSocket PTY.

### CI and tooling

- Deprecated `actions-rs/*` (v1) replaced with `dtolnay/rust-toolchain` + `Swatinem/rust-cache`
- Frontend TypeScript and build checks added to CI
- `cargo deny` supply-chain check added
- All GitHub Action versions bumped to v4/v5
- `rust-toolchain.toml` pins the exact Rust version for reproducible builds
- `deny.toml` added for license allow-listing, vulnerability checks, and duplicate-crate warnings

### Kubernetes

- Fixed the Secret placeholder in `k8s/deployment.yaml` — it was pointing at SQLite (wrong DB), now it's PostgreSQL
- Added missing manifests: `namespace.yaml`, `secrets.yaml`, `network-policy.yaml`, `pdb.yaml`

### Observability

- Grafana provisioning directory with auto-configured Prometheus datasource
- Pre-built Grafana dashboard (`grafana/dashboards/wasmos-overview.json`) covering HTTP metrics, task latency percentiles, WASM instruction counts, memory usage, and error rates

---

## 0.1.0 — January 2026

Initial release. This was the baseline we shipped at the end of Sprint 1 / early Sprint 2.

### What was included

- Actix-Web 4 REST API with 38+ endpoints across `/v1/*` and `/v2/*`
- PostgreSQL persistence using sqlx 0.7 with a repository pattern
- Custom WASM interpreter (no wasmtime/wasmer) with static security analysis
- JWT authentication middleware, per-IP rate limiting, request-ID tracing
- Capability token system for zero-trust access control
- Preemptive multi-tenant scheduler with a priority queue
- Distributed trace store with live P50/P95/P99 metric computation
- WebSocket event stream for real-time task status updates
- xterm.js terminal with PTY mode over WebSocket
- Prometheus metrics endpoint
- Grafana + Prometheus stack via docker-compose
- Plugin lifecycle hooks (logging, metrics)
- Snapshot save/restore API
- Audit log API
- Next.js 14 frontend with dashboard, tasks, terminal, metrics, tokens, and snapshots pages
- Docker multi-stage build
- Fly.io, Railway, and Render deployment configs
- Kubernetes manifests (Deployment, Service, Ingress, HPA)
