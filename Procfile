# Procfile — process declarations for Railway, Render, Heroku, and Fly.io
# ─────────────────────────────────────────────────────────────────────────────
# Railway and Render both support Procfile-based process declarations.
# The `web` process receives the platform-injected $PORT automatically.
#
# If railway.toml / render.yaml startCommand is set, those take precedence.
# This file acts as a fallback and is also used by `foreman start` locally.
#
# Install foreman for local use:  gem install foreman
#                             or: cargo install forego
# Local dev:  foreman start -f Procfile.dev
# ─────────────────────────────────────────────────────────────────────────────

web: wasmos/target/release/wasmos
