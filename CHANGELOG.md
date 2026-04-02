# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- `nixpacks.toml` — Nixpacks build configuration (replaces Dockerfile); controls nix packages, build command, and start command
- `Procfile` — platform-agnostic process declaration for Railway, Render, and Heroku-compatible platforms
- `Procfile.dev` — local multi-process development runner (backend + frontend via foreman/forego)
- `railway.json` — Railway service config (machine-readable JSON schema alongside TOML)
- `frontend/railway.json` — Railway service config for Next.js frontend service
- `DEPLOYMENT.md` — comprehensive deployment guide for Railway, Render, and Fly.io including environment variable reference, CORS wiring, and observability setup

### Changed
- **Removed Docker entirely** — `Dockerfile`, `docker-compose.dev.yml`, `docker-compose.prod.yml`, `.dockerignore`, and `nginx/` directory deleted
- `railway.toml` — expanded with full environment variable reference table, health endpoint docs, observability notes, and corrected `startCommand` path
- `render.yaml` — changed `env: docker` → `env: nixpacks`, added frontend service, added all environment variables, uses `rootDir: wasmos` with native Cargo build
- `fly.toml` — removed `dockerfile` build reference, switched to `builder = "nixpacks"`, expanded non-secret `[env]` block, added full secrets setup instructions
- `.gitignore` — added `.nixpacks/`, `.railway/`, `.fly/`, `.render-buildpacks.json`, `coverage/` ignores; removed Docker-specific comment
- `.github/workflows/ci.yml` — replaced Docker build/push job with Railway deploy jobs (backend + frontend) triggered on push to `main`

### Removed
- `Dockerfile` — no longer needed; Nixpacks builds the binary directly from source
- `docker-compose.dev.yml` — replaced by `Procfile.dev` + local Postgres
- `docker-compose.prod.yml` — replaced by cloud platform configs (Railway/Render/Fly.io)
- `.dockerignore` — irrelevant without Docker
- `nginx/` directory — TLS termination is handled by the cloud platform's edge layer

- `WasmOsError::Unauthorized` (HTTP 401) and `WasmOsError::NotFound` (HTTP 404) error variants
- Machine-readable `code` field in all error JSON responses (`NOT_FOUND`, `VALIDATION_ERROR`, …)
- DB error sanitisation — raw sqlx messages no longer reach HTTP clients
- `[SECURITY WARNING]` startup log when `admin_key`, `jwt_secret`, or CORS is set to insecure defaults
- Per-file 30s timeout in `POST /v1/test-files/run-all` to prevent infinite-loop WASM from blocking the batch
- Prometheus path-label normalisation in request-logging middleware — UUID/numeric path segments replaced with `{id}` to prevent cardinality explosion
- `.dockerignore` — excludes `target/`, `node_modules/`, test artefacts, and secrets from Docker build context
- `rust-toolchain.toml` — pins exact Rust version for reproducible builds
- `deny.toml` — supply-chain security: licence allow-list, vulnerability deny, duplicate-crate warnings
- `docker-compose.prod.yml` — production stack with Nginx, Certbot/TLS, internal-only Postgres network, `read_only` container FS, structured logging
- `k8s/namespace.yaml`, `k8s/secrets.yaml`, `k8s/network-policy.yaml`, `k8s/pdb.yaml` — missing Kubernetes manifests
- `SECURITY.md` — vulnerability reporting policy, response SLAs, security architecture summary
- `grafana/provisioning/` — auto-provisioned Prometheus datasource and dashboard provider
- `grafana/dashboards/wasmos-overview.json` — production Grafana dashboard (HTTP metrics, task latency P50/P95/P99, WASM instructions, memory, error rate)

### Changed
- `GET /v1/auth/token` now returns **401 Unauthorized** (was 400) on wrong admin key, with no hint whether key was missing vs wrong
- `TaskNotRunning` error now maps to **422 Unprocessable Entity** (was 409 Conflict)
- `delete_snapshot` / `get_snapshot` use `WasmOsError::NotFound` instead of `TaskNotFound`
- `get_tenant` 404 uses `WasmOsError::NotFound("Tenant {id}")` instead of `TaskNotFound`
- `update_task` in repository now issues a **single atomic UPDATE** (was two sequential queries — not atomic)
- `create_schema()` removed from `db/mod.rs`; `connect_pg()` applies `001_initial_schema.sql` as the single source of truth
- Duplicate `sqlx::raw_sql` migration call removed from `main.rs`
- Root `.gitignore` expanded from a single line to full coverage of build artefacts, secrets, and editor files
- CI workflow rewritten: deprecated `actions-rs/*` (v1) replaced with `dtolnay/rust-toolchain` + `Swatinem/rust-cache`; frontend TypeScript/build check added; `cargo deny` supply-chain check added; all action versions bumped to v4/v5
- `k8s/deployment.yaml` Secret placeholder fixed — was `sqlite:///...` (wrong DB), now PostgreSQL placeholder

### Fixed
- `execution_history.success` column: migration now correctly uses `NOT NULL DEFAULT FALSE` (was `DEFAULT NULL`), consistent with Rust `bool` field
- `idx_exec_history_task` replaced by `idx_exec_history_task_time (task_id, started_at DESC)` composite index
- Missing indexes added: `idx_tasks_sched` (partial, pending tasks only), `idx_exec_history_success`, `idx_audit_action`, `idx_snapshots_captured_at`

---

## [0.1.0] — 2026-01-01

### Added
- Initial production release
- Actix-web 4 REST API (`/v1/*`) with 38 endpoints
- PostgreSQL persistence via sqlx 0.7 with repository pattern
- Custom WASM interpreter with static security analysis
- JWT authentication middleware, per-IP rate limiting, request-ID logging
- Capability token system (zero-trust access control)
- Preemptive multi-tenant scheduler with priority queue
- Distributed trace store with live P50/P95/P99 metrics
- WebSocket live event stream + xterm.js PTY terminal mode
- Prometheus metrics endpoint + Grafana/Prometheus stack in docker-compose
- Plugin lifecycle hooks
- Snapshot save/restore API
- Audit log API
- Next.js 14 frontend with real-time dashboard, tasks, terminal, metrics, tokens, snapshots pages
- Docker multi-stage build, Fly.io / Railway / Render deployment configs
- Kubernetes Deployment + Service + Ingress + HPA manifests
