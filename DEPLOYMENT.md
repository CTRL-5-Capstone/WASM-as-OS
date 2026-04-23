# Deploying WasmOS

We moved away from Docker a while back. The project now builds with **Nixpacks**, which all three of our supported platforms (Railway, Render, Fly.io) understand natively. The config files in the repo root handle most of the setup for you.

## Which platform should I use?

Honestly, all three work fine. Here's a quick comparison:

| | Railway | Render | Fly.io |
|---|---|---|---|
| Config files | `railway.toml`, `railway.json` | `render.yaml` | `fly.toml` |
| Build system | Nixpacks | Nixpacks | Nixpacks |
| Managed Postgres | Built-in | Built-in | Via `fly postgres` |
| WebSocket support | Yes | Yes | Yes |
| How you deploy | `railway up` | Blueprint (or `render blueprint apply`) | `fly deploy` |

Railway is probably the simplest if you've never deployed a Rust project before. Fly.io gives you the most control.

## Railway

Install the CLI and log in:

```bash
npm install -g @railway/cli
railway login
```

Set up the project and database:

```bash
railway init
railway add --database postgres
```

Configure secrets (on macOS/Linux — on Windows just generate random strings however you like and paste them):

```bash
railway variables set \
  WASMOS__SECURITY__JWT_SECRET="$(openssl rand -base64 48)" \
  WASMOS__SECURITY__ADMIN_KEY="$(openssl rand -hex 32)" \
  WASMOS__SECURITY__AUTH_ENABLED=true \
  WASMOS__SERVER__HOST=0.0.0.0 \
  WASMOS__SERVER__PORT='${{PORT}}' \
  WASMOS__DATABASE__URL='${{Postgres.DATABASE_URL}}' \
  WASMOS__LOGGING__FORMAT=json \
  WASMOS__LOGGING__LEVEL=info
```

Deploy:

```bash
railway up
```

For CI/CD, add a `RAILWAY_TOKEN` secret to your GitHub repo (get one from `railway whoami --token`). The workflow in `.github/workflows/ci.yml` triggers on pushes to `main`.

## Render

The easiest way is a Blueprint deploy:

1. Push this repo to GitHub
2. In the Render dashboard: **New → Blueprint**
3. Connect your repo — Render reads `render.yaml` and sets everything up

If you prefer the CLI:

```bash
npm install -g @render-com/cli
render login
render blueprint apply
```

## Fly.io

Install the CLI and authenticate:

```bash
curl -L https://fly.io/install.sh | sh
fly auth login
```

First-time setup:

```bash
fly launch --no-deploy

# Create and attach a Postgres cluster
fly postgres create --name wasmos-db --region iad
fly postgres attach wasmos-db

# Set secrets
fly secrets set \
  WASMOS__SECURITY__JWT_SECRET="$(openssl rand -base64 48)" \
  WASMOS__SECURITY__ADMIN_KEY="$(openssl rand -hex 32)"

fly deploy
```

## Running locally with Foreman (no cloud)

If you just want one command to start both the backend and frontend:

```bash
gem install foreman
cp .env.example .env
foreman start -f Procfile.dev
```

This starts the backend at `http://localhost:8080` and the frontend at `http://localhost:3001`.

## Environment variables

Here's what the backend reads from the environment. Most of these have sensible defaults for development, but you'll want to set the security ones properly before deploying anywhere real.

| Variable | Needed in prod? | Default | What it does |
|---|---|---|---|
| `WASMOS__DATABASE__URL` | Yes | *(none)* | PostgreSQL connection string |
| `WASMOS__SECURITY__JWT_SECRET` | Yes | *(none)* | Used to sign JWT tokens — pick something long and random |
| `WASMOS__SECURITY__ADMIN_KEY` | Yes | *(none)* | Required to mint tokens via `POST /v1/auth/token` |
| `WASMOS__SECURITY__AUTH_ENABLED` | Yes | `false` | Turns on JWT middleware for all `/v1/*` routes |
| `WASMOS__SERVER__HOST` | Sometimes | `127.0.0.1` | Set to `0.0.0.0` on cloud platforms |
| `WASMOS__SERVER__PORT` | Sometimes | `8080` | Most platforms inject a `PORT` variable |
| `WASMOS__SERVER__CORS_ORIGINS` | Sometimes | `*` | Lock this down to your frontend URL in production |
| `WASMOS__LOGGING__FORMAT` | No | `pretty` | Use `json` in production for structured logging |
| `WASMOS__LOGGING__LEVEL` | No | `info` | One of `trace`, `debug`, `info`, `warn`, `error` |

## Health checks and metrics

| Endpoint | What it tells you |
|---|---|
| `GET /health/live` | The process is alive |
| `GET /health/ready` | The process is alive *and* the database is reachable |
| `GET /metrics` | Prometheus-format metrics (request latency, task counts, WASM instructions, memory usage) |

A pre-built Grafana dashboard is included at `grafana/dashboards/wasmos-overview.json` if you want to plug it into your monitoring stack.
