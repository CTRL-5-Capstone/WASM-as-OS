# WasmOS Backend

This is the Rust backend for WasmOS. It handles the REST API, WebSocket connections, WASM execution, task scheduling, and (when the `web/` directory exists) serves the static dashboard UI.

The WASM execution engine is written from scratch — there's no wasmtime or wasmer in the dependency tree. We built a custom interpreter partly to learn how it works and partly to have full control over what host functions are available to guest modules.

## Running it

From the repo root:

```powershell
cd wasmos
cargo run
```

First build takes a while (lots of dependencies to compile). Once it's up:

- Health check: `http://127.0.0.1:8080/health/live`
- Readiness (includes DB): `http://127.0.0.1:8080/health/ready`
- Prometheus metrics: `http://127.0.0.1:8080/metrics`
- WebSocket: `ws://127.0.0.1:8080/ws`
- API docs start at: `http://127.0.0.1:8080/v1/tasks`

If you've built the frontend (see `../frontend/README.md`), the static export lives in `../web/` and the server will serve it at `http://127.0.0.1:8080/`.

## Project structure

Here's a rough map of what lives where:

| Path | What it does |
|---|---|
| `src/main.rs` | Server startup, configuration loading, wiring everything together |
| `src/server.rs` | All the HTTP route handlers (1800+ lines — it's a big file) |
| `src/advanced_execution_endpoints.rs` | v2 API endpoints (batch execution, module management, performance comparison) |
| `src/run_wasm/` | The custom WASM interpreter and execution engine |
| `src/db/` | Database models, repository pattern, connection setup |
| `src/middleware/` | JWT auth, rate limiting, request ID injection |
| `src/scheduler.rs` | Preemptive task scheduler with priority queue |
| `src/websocket.rs` | WebSocket endpoint for live task events and terminal PTY |
| `src/capability.rs` | Zero-trust capability token registry |
| `src/tracing_spans.rs` | In-memory distributed trace store |
| `src/tenancy.rs` | Multi-tenant resource isolation and quota enforcement |
| `src/plugins.rs` | Plugin lifecycle hooks (logging, metrics) |
| `src/query_cache.rs` | In-memory query result cache (moka, TTL-based) |
| `src/metrics.rs` | Prometheus metric definitions |
| `src/telemetry.rs` | Tracing/logging initialization |
| `src/config.rs` | Configuration loading from env vars and TOML |
| `migrations/` | PostgreSQL schema migrations (001 through 004) |

## Database

The backend uses PostgreSQL. Migrations run automatically on startup via `sqlx::migrate!()`. The schema covers six tables:

- `tasks` — uploaded WASM modules with status, priority, and tenant assignment
- `task_metrics` — aggregate execution stats per task
- `execution_history` — individual execution records with timing and error info
- `snapshots` — saved WASM execution states (memory, stack, globals)
- `tenants` — multi-tenant isolation boundaries with resource quotas
- `audit_log` — every API action with user, role, IP, and timestamp

If Postgres isn't available at startup, the server still runs but persistence is disabled and some endpoints return 503.

## API overview

There are 40+ endpoints spread across v1 and v2. The main ones:

- **Tasks:** CRUD, start/stop/pause/restart, execution history, security analysis, logs
- **Snapshots:** create, list, delete, get by ID
- **Traces:** list recent, get by task, live P50/P95/P99 metrics
- **Tokens:** issue, list, revoke, capability check
- **Tenants:** create, list, delete
- **Audit:** filtered log with pagination
- **Scheduler:** status, preempt
- **Auth:** JWT token issuance
- **v2 advanced:** batch execution, module management, performance comparison, import stats, deep inspection

## Team

**CTRL-5 Capstone:**

- Ololade Awoyemi
- Benjamin Wilson
- Biraj Sharma
- Shivam Sakthivel Pandi
- Sritan Reddy Gangidi
