#!/bin/sh
# docker/fke-server/entrypoint.sh — Flint Kiln (fke-server) container entrypoint.
#
# Reads Docker secret files and builds environment variables before exec-ing
# the binary. Eliminates the need for a .env file on production hosts.
#
# Secret mounts (from docker-compose.prod.yml secrets:):
#   /run/secrets/postgres_password  → used to build DATABASE_URL
#
# Fallback: if the secret file is absent (local dev / non-prod), the environment
# variable already set in the container is preserved unchanged.
set -e

# ── postgres_password → DATABASE_URL ─────────────────────────────────────────
if [ -r /run/secrets/postgres_password ]; then
    PG_PASS=$(cat /run/secrets/postgres_password)
    export DATABASE_URL="postgres://flint:${PG_PASS}@db:5432/flint"
fi

# ── exec binary — pass through all arguments ─────────────────────────────────
exec /usr/local/bin/fke-server "$@"
