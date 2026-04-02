# WasmOS backend (`wasmos`)

This crate is the Rust backend for WasmOS. It exposes REST APIs, a WebSocket endpoint, and (when `web/` is present) serves the static dashboard UI.

The WASM execution engine is implemented as a custom interpreter in this repo (no `wasmtime`/`wasmer`).

## Local run

From the repo root:

```powershell
cd .\wasmos
cargo run
```

Useful URLs:

- `GET http://127.0.0.1:8080/health/live`
- `GET http://127.0.0.1:8080/health/ready`
- `GET http://127.0.0.1:8080/metrics`
- `WS  ws://127.0.0.1:8080/ws`

If you build the frontend (see `../frontend/README.md`), the exported static site lands in `../web/`, and this server will serve it at:

- http://127.0.0.1:8080/

## Code map

- `src/main.rs`: server startup and wiring
- `src/server.rs`: HTTP API routes and handlers
- `src/websocket.rs`: WebSocket endpoint
- `src/run_wasm/`: custom interpreter and execution engine
- `migrations/`: SQL migrations for Postgres

## Authors

Team CTRL 5:

- Ololade Awoyemi
- Benjamin Wilson
- Biraj Sharma
- Shivam Sakthivel Pandi
- Sritan Reddy Gangidi
