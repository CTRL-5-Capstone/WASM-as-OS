# Deployment guide

This repo is set up to build with Nixpacks (no Docker required). The provided config files support Railway, Render, and Fly.io.

## Platform comparison

| | Railway | Render | Fly.io |
|---|---|---|---|
| Config | `railway.toml`, `railway.json` | `render.yaml` | `fly.toml` |
| Build system | Nixpacks | Nixpacks | Nixpacks |
| Managed Postgres | Yes | Yes | Yes (via `fly postgres`) |
| WebSockets | Yes | Yes | Yes |
| Deploy style | `railway up` | Blueprint | `fly deploy` |

## Railway

Prerequisites:

```bash
npm install -g @railway/cli
railway login
```

Deploy (single service: API + UI):

```bash
railway init
railway add --database postgres

# Note: `openssl` is commonly available on macOS/Linux.
# On Windows, generate secrets however you prefer and paste the values.
railway variables set \
  WASMOS__SECURITY__JWT_SECRET="$(openssl rand -base64 48)" \
  WASMOS__SECURITY__ADMIN_KEY="$(openssl rand -hex 32)" \
  WASMOS__SECURITY__AUTH_ENABLED=true \
  WASMOS__SERVER__HOST=0.0.0.0 \
  WASMOS__SERVER__PORT='${{PORT}}' \
  WASMOS__DATABASE__URL='${{Postgres.DATABASE_URL}}' \
  WASMOS__LOGGING__FORMAT=json \
  WASMOS__LOGGING__LEVEL=info

railway up
```

CI/CD:

- Add a GitHub secret `RAILWAY_TOKEN` (get it from `railway whoami --token`).
- `.github/workflows/ci.yml` deploys on pushes to `main`.

## Render

Blueprint deploy:

1. Push this repo to GitHub
2. In Render: New -> Blueprint
3. Connect your repo (Render reads `render.yaml`)

Manual deploy:

```bash
npm install -g @render-com/cli
render login
render blueprint apply
```

## Fly.io

Prerequisites:

```bash
curl -L https://fly.io/install.sh | sh
fly auth login
```

First deploy:

```bash
fly launch --no-deploy

fly postgres create --name wasmos-db --region iad
fly postgres attach wasmos-db

fly secrets set \
  WASMOS__SECURITY__JWT_SECRET="$(openssl rand -base64 48)" \
  WASMOS__SECURITY__ADMIN_KEY="$(openssl rand -hex 32)"

fly deploy
```

## Local multi-process dev (no cloud)

If you want a single command to start backend + frontend together, you can use `Procfile.dev` with Foreman/forego:

```bash
gem install foreman
cp .env.example .env
foreman start -f Procfile.dev
```

This starts:

- Backend: `http://127.0.0.1:8080/`
- Frontend: `http://127.0.0.1:3001/`

## Environment variables

| Variable | Required in production | Default | Description |
|---|---|---|---|
| `WASMOS__DATABASE__URL` | Yes | (none) | PostgreSQL connection string |
| `WASMOS__SECURITY__JWT_SECRET` | Yes | (none) | Random secret for JWT signing |
| `WASMOS__SECURITY__ADMIN_KEY` | Yes | (none) | Admin key for minting tokens |
| `WASMOS__SECURITY__AUTH_ENABLED` | Usually | `false` | Enable auth middleware |
| `WASMOS__SERVER__HOST` | No | `127.0.0.1` | Use `0.0.0.0` on cloud |
| `WASMOS__SERVER__PORT` | No | `8080` | Use platform `PORT` |
| `WASMOS__SERVER__CORS_ORIGINS` | No | `*` | Set to your frontend URL |
| `WASMOS__LOGGING__FORMAT` | No | `pretty` | Use `json` in production |
| `WASMOS__LOGGING__LEVEL` | No | `info` | `trace`/`debug`/`info`/`warn`/`error` |

## Health and metrics

| Endpoint | Purpose |
|---|---|
| `GET /health/live` | Liveness |
| `GET /health/ready` | Readiness (includes DB status) |
| `GET /metrics` | Prometheus scrape endpoint |

## Observability

Prometheus metrics are available at `/metrics`. A Grafana dashboard JSON is included at `grafana/dashboards/wasmos-overview.json`.
