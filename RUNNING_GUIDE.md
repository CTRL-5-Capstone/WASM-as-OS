# Running WasmOS (full stack)

This repo has two common local workflows:

1) Local dev: backend on `:8080` + Next dev server on `:3001` (proxying API calls)
2) Production-style: build a static UI into `web/` and let the Rust backend serve it

## Prerequisites

- Rust (stable) + Cargo
- Node.js 20+ + npm
- PostgreSQL 15+ (optional, but recommended)

If Postgres is not available, `wasmos` should still start, but persistence is disabled and some endpoints may return `503`.

## Option A: one-command local start (Windows PowerShell)

From the repo root:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\start-local-dev.ps1
```

Stop everything:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\stop-local-dev.ps1
```

## Option B: start services manually (Windows PowerShell)

### 1) Start the backend on :8080

```powershell
cd .\wasmos

# Optional: point to a different Postgres instance
# $env:WASMOS__DATABASE__URL = "postgresql://postgres:postgres@localhost:5432/wasmos"

cargo run
```

Health check:

```powershell
curl.exe http://127.0.0.1:8080/health/live
```

### 2) Start the dashboard on :3001

In a second terminal:

```powershell
cd .\frontend

npm install
npm run dev
```

Open:

- http://127.0.0.1:3001/

## Verify local integration

With the backend running on `:8080` and the frontend on `:3001`:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\verify-local-dev.ps1
```

## Production-style local run (backend serves the UI)

This builds a static export into `web/` (via `frontend/next.config.mjs`), then serves it from the Rust backend.

```powershell
cd .\frontend
npm install
npm run build

cd ..\wasmos
cargo run
```

Open:

- http://127.0.0.1:8080/

## Deploy

See `DEPLOYMENT.md` for Railway / Render / Fly.io.
