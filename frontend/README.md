# WasmOS dashboard (Next.js)

This is the React dashboard for WasmOS, built with Next.js (App Router).

## How it talks to the backend

In development (`npm run dev`), the dashboard proxies requests to the Rust backend using rewrites in `next.config.mjs`. That lets the browser call:

- `/v1/*`
- `/v2/*`
- `/health/*`
- `/metrics`
- `/ws`

…without needing CORS.

In production-style builds (`npm run build`), Next exports static files directly into `../web/` so the Rust backend can serve the UI and APIs from one origin.

## Local development (Windows PowerShell)

1) Start the Rust backend on `:8080`:

```powershell
cd ..\wasmos
cargo run
```

2) Start the dashboard on `:3001`:

```powershell
cd ..\frontend
npm install
npm run dev
```

Open:

- http://127.0.0.1:3001/

## Point the dev proxy at a different backend (optional)

```powershell
$env:NEXT_PUBLIC_BACKEND_URL = "http://127.0.0.1:8080"
npm run dev
```

## Build a static UI for the Rust backend

```powershell
npm install
npm run build
```

This writes the static export to `../web/`.
