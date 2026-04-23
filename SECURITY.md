# Security

## Supported versions

We only actively maintain the `main` branch. If you're running an older checkout or tagged release, please pull the latest before reporting issues — there's a decent chance it's already been fixed.

| Version | Supported |
|---|---|
| `main` | Yes |
| Anything else | Not actively — please upgrade |

## Reporting a vulnerability

**Don't open a public GitHub issue for security bugs.** We'd rather fix it before it's out in the open.

The best way to report is through [GitHub's private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing/privately-reporting-a-security-vulnerability). That lets us triage it privately and coordinate a fix without broadcasting the details.

When you report something, it helps if you can include:

- A clear description of the problem and why it matters
- Steps to reproduce — even a rough outline is better than nothing
- What part of the system is affected (backend, frontend, deployment config, Kubernetes, etc.)
- If you've thought about a fix or mitigation, we'd love to hear it

## How quickly we aim to respond

These are targets, not promises. We're a small team and sometimes things take a bit longer.

| Severity | First response | Fix target |
|---|---|---|
| Critical (RCE, auth bypass, data leak) | 24 hours | 72 hours |
| High (privilege escalation, significant data exposure) | 48 hours | 7 days |
| Medium (information disclosure, CSRF, etc.) | 5 business days | 30 days |
| Low (minor hardening, defense-in-depth) | 14 days | Next planned release |

## How the security model works

Here's a quick summary of the layers we've put in place. None of these are silver bullets on their own, but together they cover a reasonable amount of ground.

### Authentication and authorization

When `WASMOS__SECURITY__AUTH_ENABLED=true` (which you should always set in production), all `/v1/*` routes require a valid JWT in the `Authorization` header. Tokens are issued via `POST /v1/auth/token` using an admin key that you configure through `WASMOS__SECURITY__ADMIN_KEY`.

On top of JWTs, there's a **capability token** system for more granular access control. You can issue tokens with specific permissions like `task_read`, `task_execute`, `snapshot_write`, etc., and optionally scope them to a specific tenant. Tokens can have expiry times and can be revoked individually.

### WASM sandboxing

All WebAssembly modules run through our custom interpreter (under `wasmos/src/run_wasm/`). There's no `wasmtime` or `wasmer` in the dependency tree — we wrote the execution engine from scratch, partly as a learning exercise and partly so we'd have full control over what host functions are exposed.

Static security analysis is available at `GET /v1/tasks/{id}/security`, which parses the binary's import/export sections and flags anything that looks suspicious (file I/O, network access, process spawning, etc.). The frontend's Security page takes this further with control-flow graph visualization, entropy analysis, and pattern matching.

### Path traversal protection

Task file paths stored in the database are validated against the `wasm_files/` directory before any `fs::read` or execution happens. Even if someone manages to inject a `../../etc/passwd` path into the database, the backend canonicalizes it and checks that it's inside the sandbox before touching the filesystem.

### Supply chain

We use `cargo deny` in CI to check for known vulnerabilities, license violations, and duplicate crates. The Rust toolchain version is pinned in `wasmos/rust-toolchain.toml` so builds are reproducible.

### Rate limiting and network policy

The backend has a per-IP rate limiter controlled by `WASMOS__SECURITY__RATE_LIMIT_PER_MINUTE`. For Kubernetes deployments, a NetworkPolicy is provided at `k8s/network-policy.yaml` to restrict pod-to-pod traffic.

### Secrets management

We don't commit secrets. Environment variables are the expected mechanism, and `.env` files are gitignored. The `k8s/secrets.yaml` in the repo contains placeholder values that you're meant to replace.

If you start the server with weak defaults for `admin_key` or `jwt_secret`, it prints a warning at startup. Don't ignore that warning in production.

## Known limitations

- With `AUTH_ENABLED=false` (the default for local dev), there's no authentication at all. This is intentional for development convenience but obviously shouldn't be deployed that way.
- The audit log records API actions but doesn't currently capture WebSocket events.
- The WASM interpreter is a learning project — it hasn't been through a formal security audit. Don't run untrusted WASM in production without understanding the risks.
