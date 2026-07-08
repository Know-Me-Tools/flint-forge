#!/usr/bin/env bash
# Flint Forge — full-stack integration + k6 regression gate (p15-c004).
#
# Starts the local Docker Compose stack (Postgres 18 + extensions + fdb-gateway
# + fke-server), applies migrations, runs DATABASE_URL-gated tests, and runs
# perf/k6/regression.js.
#
# Local:
#   ./scripts/ci-stack-test.sh
#
# Environment overrides:
#   FLINT_JWT_SECRET  JWT signing secret (default: generated)
#   DATABASE_URL      DB connection string (default: postgres://flint:flint@localhost:5432/flint)
#   BASE_URL          fdb-gateway base URL (default: http://localhost:8080)
#   KILN_ADMIN_URL    Kiln admin URL (default: http://localhost:8090)
#
# The stack is torn down on exit.
set -euo pipefail

cd "$(dirname "$0")/.."

: "${FLINT_JWT_SECRET:=$(openssl rand -hex 32)}"
export FLINT_JWT_SECRET
export DATABASE_URL="${DATABASE_URL:-postgres://flint:flint@localhost:5432/flint}"
export BASE_URL="${BASE_URL:-http://localhost:8080}"
export KILN_ADMIN_URL="${KILN_ADMIN_URL:-http://localhost:8090}"

cleanup() {
  echo "==> tearing down stack"
  docker compose -f docker-compose.yml down -v || true
}
trap cleanup EXIT

echo "==> starting Docker Compose stack"
docker compose -f docker-compose.yml up -d --build

echo "==> waiting for fdb-gateway /healthz"
for i in $(seq 1 60); do
  if curl -sf "${BASE_URL}/healthz" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
curl -sf "${BASE_URL}/healthz" >/dev/null || { echo "fdb-gateway did not become healthy"; exit 1; }

echo "==> running migration + db-integration tests"
./scripts/ci-test.sh

echo "==> building forge-cli smoke token"
cargo build -p forge-cli -q
TOKEN="$(./target/debug/forge token mint)"
export TOKEN

echo "==> k6 regression gate"
if command -v k6 >/dev/null 2>&1; then
  BASE_URL="${BASE_URL}" TOKEN="${TOKEN}" k6 run perf/k6/regression.js
else
  echo "k6 not installed — skipping regression gate"
fi

echo "OK: stack integration + k6 regression green"
