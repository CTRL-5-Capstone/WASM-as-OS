# WasmOS Dashboard

This is the web frontend for WasmOS — a Next.js 14 app (App Router) using React 18, TypeScript, Tailwind CSS, and shadcn/ui components.

## How it connects to the backend

In development, the Next.js dev server proxies API requests to the Rust backend using rewrites defined in `next.config.mjs`. From the browser's perspective, everything lives on the same origin (localhost:3001), so there's no CORS to deal with.

The proxied routes are:

- `/v1/*` and `/v2/*` — REST API
- `/health/*` — health checks
- `/metrics` — Prometheus endpoint
- `/ws` — WebSocket for real-time events and the terminal

In production builds, Next exports static files into `../web/`. The Rust backend serves those alongside the API from a single port, so there's no proxy layer at all.

## Running locally

You'll need the Rust backend running on port 8080 first (see `../wasmos/README.md`), then:

```powershell
cd frontend
npm install
npm run dev
```

Open [http://localhost:3001](http://localhost:3001).

If you need to point the proxy at a different backend:

```powershell
$env:NEXT_PUBLIC_BACKEND_URL = "http://127.0.0.1:8080"
npm run dev
```

## Building for production

```powershell
npm install
npm run build
```

This writes the static export to `../web/`. Start the Rust backend and open `http://localhost:8080`.

## What's in the dashboard

The frontend has quite a few pages at this point:

- **Dashboard** (`/`) — system stats, recent activity, health overview
- **Tasks** (`/tasks`) — upload, run, and manage WASM modules; environment simulation tools (ABI mocking, vFS explorer, env vars, scenario orchestrator)
- **Terminal** (`/terminal`) — xterm.js shell connected over WebSocket
- **Traces** (`/traces`) — distributed tracing with waterfall charts, environment-aware heatmaps, forensic snapshots, regression test cloning
- **Snapshots** (`/snapshots`) — capture/restore execution state, environment sidecar metadata, memory-vFS mapping, time-travel diffing, forking
- **Security** (`/security`) — binary analysis, control-flow graphs, entropy visualization, YARA pattern matching
- **Metrics** (`/metrics`) — live Prometheus metrics and per-task breakdowns
- **Audit** (`/audit`) — action log with filtering and playback
- **Tokens** (`/tokens`) — capability token management (issue, revoke, inspect)
- **RBAC** (`/rbac`) — tenant management and role-based access control
- **Batch** (`/batch`) — run multiple WASM files at once
- **Monitor** (`/monitor`) — real-time system health and scheduler status
- **Tests** (`/tests`) — test file discovery and execution
- **Analytics** (`/analytics`) — execution trends and performance analysis
