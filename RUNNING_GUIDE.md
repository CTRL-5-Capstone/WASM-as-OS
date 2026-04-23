# Running WasmOS locally

There are two ways to run this project on your machine — pick whichever fits your workflow.

**Option A** is the day-to-day development setup: the Rust backend runs on port 8080 and the Next.js dev server runs on port 3001, proxying API calls to the backend behind the scenes. You get hot-reload on the frontend and can iterate fast.

**Option B** is the production-style setup: you build the dashboard into static files, drop them into `web/`, and the Rust server serves everything from one port. This is closer to what actually runs in deployment.

## What you'll need

- **Rust** (stable toolchain) — `rustup` will handle this; we pin the version in `wasmos/rust-toolchain.toml`
- **Node.js 20+** and npm
- **PostgreSQL 15+** — technically optional, but you'll want it. Without a database the backend still starts, but task persistence is disabled and some endpoints will return 503.

## Option A: development mode (two terminals)

### Start the backend

Open a terminal at the repo root:

```powershell
cd .\wasmos
cargo run
```

First build takes a few minutes. Once it's up you should see `WASM-OS is ready!` in the output. Quick sanity check:

```powershell
curl.exe http://127.0.0.1:8080/health/live
```

If you need to point at a specific Postgres instance:

```powershell
$env:WASMOS__DATABASE__URL = "postgresql://postgres:postgres@localhost:5432/wasmos"
cargo run
```

### Start the dashboard

Open a second terminal:

```powershell
cd .\frontend
npm install
npm run dev
```

Then open [http://localhost:3001](http://localhost:3001) in your browser. The Next.js dev server takes care of routing `/v1/*`, `/v2/*`, `/health/*`, `/metrics`, and `/ws` to the backend — you don't need to worry about CORS.

### One-command alternative (PowerShell scripts)

If you'd rather not juggle two terminals, we have scripts for that:

```powershell
# Start both services
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\start-local-dev.ps1

# Stop everything
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\stop-local-dev.ps1
```

Or if you have Foreman/forego installed:

```bash
gem install foreman
cp .env.example .env
foreman start -f Procfile.dev
```

## Option B: production-style (single origin)

This builds the frontend into static HTML/JS/CSS, writes it to `web/`, and lets the Rust backend serve the whole thing.

```powershell
cd .\frontend
npm install
npm run build

cd ..\wasmos
cargo run
```

Open [http://localhost:8080](http://localhost:8080) — that's both the UI and the API on one port.

## Verifying everything works

With services running (either option), you can run the local integration check:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\verify-local-dev.ps1
```

This hits the health endpoints, creates a test task, runs it, and cleans up.

## Deploying to the cloud

See [DEPLOYMENT.md](DEPLOYMENT.md) for Railway, Render, and Fly.io instructions.
