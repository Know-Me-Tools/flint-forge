#!/usr/bin/env bash
# Flint Forge — test runner (p35-c003).
#
# Two stages:
#   1. unit          — always runs; no database required.
#   2. db-integration — runs ONLY when DATABASE_URL is set; applies migrations
#                       then runs the full test set (incl. DATABASE_URL-gated tests).
#
# Local:  DATABASE_URL=postgres://user@localhost/flint  ./scripts/ci-test.sh
# CI:     the Dagger CheckDb service binding exports DATABASE_URL, then calls this.
#
# The database must have `vector` (pgvector) and `pg_graphql` available — use the
# pinned docker/postgres image. See docker/postgres/Dockerfile.
set -euo pipefail

echo "==> verify migrations"
./scripts/verify-migrations.sh migrations

echo "==> unit tests (no DB)"
# --lib/--bins across the workspace: pure logic; DATABASE_URL-gated integration
# tests live in tests/ and are handled in the db stage below.
cargo test --workspace --lib --bins

if command -v docker >/dev/null 2>&1; then
  echo "==> build forge-cli container image"
  docker build -t flint-forge-cli -f crates/forge-cli/Dockerfile .
fi

if [[ -z "${DATABASE_URL:-}" ]]; then
  echo "==> db-integration: SKIPPED (DATABASE_URL not set)"
  echo "OK: unit tests green (db-integration skipped — set DATABASE_URL to run it)"
  exit 0
fi

echo "==> applying migrations to \$DATABASE_URL"
if command -v sqlx >/dev/null 2>&1; then
  sqlx migrate run --source migrations
else
  # Fallback: apply each migration in order via psql (no sqlx-cli in the image).
  for f in migrations/*.sql; do
    echo "    applying ${f}"
    psql "${DATABASE_URL}" -v ON_ERROR_STOP=1 -f "${f}"
  done
fi

echo "==> db-integration tests (DATABASE_URL-gated)"
# DB-gated tests in src/ and tests/ skip gracefully when DATABASE_URL is unset.
# #[ignore]d live-Postgres tests are run explicitly below.
cargo test --workspace

echo "==> live Postgres LISTEN/NOTIFY tests"
cargo test -p fdb-realtime --test listen_live_pg -- --ignored

echo "OK: unit + db-integration tests green"
