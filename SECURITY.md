# Security policy

## Supported versions

| Version | Supported |
|---|---|
| `main` | Yes |
| Older branches/tags | No (please upgrade) |

## Reporting a vulnerability

Please do not open a public GitHub issue for security vulnerabilities.

Preferred: use GitHub private vulnerability reporting.

- https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing/privately-reporting-a-security-vulnerability

If you operate a private security inbox for this project, you can also accept reports by email (document the address here).

Include:

- What the issue is and why it matters
- Steps to reproduce (commands or a minimal PoC)
- What area is affected (backend, frontend, deployment config, k8s, etc.)
- Any suggested fix or mitigation (if you have one)

## Response targets

These targets are goals, not guarantees.

| Severity | Initial response | Target patch |
|---|---:|---:|
| Critical | 24 hours | 72 hours |
| High | 48 hours | 7 days |
| Medium | 5 days | 30 days |
| Low | 14 days | Next release |

## Security architecture (high level)

### Authentication

- When `WASMOS__SECURITY__AUTH_ENABLED=true`, auth is applied to `/v1/*`.
- Tokens are minted via `POST /v1/auth/token` (requires `WASMOS__SECURITY__ADMIN_KEY`).

### Supply chain

- `cargo deny` is used in CI (see `wasmos/deny.toml`).
- The Rust toolchain is pinned (see `wasmos/rust-toolchain.toml`).

### Secrets

- Do not commit secrets. Use environment variables or your platform secrets manager.
- `.env`-style files are gitignored.
- `k8s/secrets.yaml` contains placeholders.

### Network and abuse controls

- A Kubernetes NetworkPolicy is provided at `k8s/network-policy.yaml`.
- The backend includes a per-IP rate limiter controlled by `WASMOS__SECURITY__RATE_LIMIT_PER_MINUTE`.

### WASM sandbox

- WASM modules are executed by the custom interpreter under `wasmos/src/run_wasm/`.
- Static checks are available at `GET /v1/tasks/{id}/security`.

## Notes and limitations

- `WASMOS__SECURITY__AUTH_ENABLED=false` is intended for local development; enable auth in production.
- If you run with a weak `admin_key`/JWT secret, the server will warn at startup.
