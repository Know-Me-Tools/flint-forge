#!/bin/sh
# docker/fdb-gateway/entrypoint.sh — Flint Quarry (fdb-gateway) container entrypoint.
#
# Reads Docker secret files and builds environment variables before exec-ing
# the binary. Eliminates the need for a .env file on production hosts.
#
# Secret mounts (from docker-compose.prod.yml secrets:):
#   /run/secrets/postgres_password  → used to build DATABASE_URL
#   /run/secrets/jwt_secret         → FLINT_JWT_SECRET
#
# Fallback: if secrets are absent (local dev / non-prod), environment variables
# already set in the container are preserved unchanged. This lets the base
# docker-compose.yml continue to work with DATABASE_URL from .env.
set -e

# ── postgres_password → DATABASE_URL ─────────────────────────────────────────
# Only override DATABASE_URL when the secret file is present and readable.
# Preserves the value from docker-compose.yml / .env in local dev mode.
if [ -r /run/secrets/postgres_password ]; then
    PG_PASS=$(cat /run/secrets/postgres_password)
    export DATABASE_URL="postgres://flint:${PG_PASS}@db:5432/flint"
fi

# ── jwt_secret → FLINT_JWT_SECRET ────────────────────────────────────────────
if [ -r /run/secrets/jwt_secret ]; then
    export FLINT_JWT_SECRET=$(cat /run/secrets/jwt_secret)
fi

# ── exec binary — pass through all arguments ─────────────────────────────────
exec /usr/local/bin/fdb-gateway "$@"
