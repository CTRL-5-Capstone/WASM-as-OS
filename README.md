# WasmOS (WASM-as-OS)

WasmOS is a Rust (actix-web) service for running and managing WebAssembly tasks. It includes a scheduler, REST APIs, a WebSocket endpoint, and a Next.js dashboard.

## How this repo is meant to run

- Development: run the backend (`wasmos`) on `:8080` and the Next dev server (`frontend`) on `:3001`. The dev server proxies API calls to the backend.
- Production-style: build the dashboard as a static export into `web/`, then the backend serves the UI and APIs from a single origin.

## Repo layout

- `wasmos/`: Rust backend (API + WebSocket + serves static UI)
- `frontend/`: Next.js dashboard source
- `web/`: generated static UI output (do not edit by hand)
- `scripts/`: local start/stop and verification scripts
- `k8s/`, `helm/`: Kubernetes deployment options
- `grafana/`, `prometheus.yml`: observability assets

## Quick start

- Local dev (backend + Next dev server): see `RUNNING_GUIDE.md`
- Deployment (Railway / Render / Fly.io): see `DEPLOYMENT.md`
- Smoke tests:
	- Windows PowerShell: `scripts/verify-railway.ps1`
	- Bash: `scripts/verify-railway.sh`
- Security policy: see `SECURITY.md`

