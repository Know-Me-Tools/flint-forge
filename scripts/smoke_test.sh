#!/usr/bin/env bash
# scripts/smoke_test.sh — post-deploy health and functional checks for Flint Forge.
#
# Usage:
#   BASE_URL=http://localhost:8080 KILN_URL=http://localhost:8090 \
#   SMOKE_TOKEN=<jwt> ./scripts/smoke_test.sh
#
# Environment variables:
#   BASE_URL      fdb-gateway base URL  (default: http://localhost:8080)
#   KILN_URL      fke-server base URL   (default: http://localhost:8090)
#   SMOKE_TOKEN   JWT bearer token for authenticated endpoints (default: empty string)
#   TIMEOUT       curl connect+max-time in seconds (default: 10)
#
# Exit codes:
#   0  all checks passed
#   1  one or more checks failed (first failure stops the script via set -e)
#
# Each check prints "==> <name>  [<http_code>]  OK" on success.
# On failure it prints "FAIL: <name>" and exits 1 immediately.
set -euo pipefail

BASE="${BASE_URL:-http://localhost:8080}"
KILN="${KILN_URL:-http://localhost:8090}"
TOKEN="${SMOKE_TOKEN:-}"
TIMEOUT="${TIMEOUT:-10}"

PASS=0
FAIL=0

# ─── helpers ─────────────────────────────────────────────────────────────────

# http_code NAME URL [extra curl args...]
# Prints status line and returns the HTTP code via stdout.
http_code() {
    local name="$1"; shift
    local url="$1";  shift
    local code
    code=$(curl -sf --connect-timeout "$TIMEOUT" --max-time "$TIMEOUT" \
           -o /dev/null -w "%{http_code}" "$@" "$url" 2>/dev/null || true)
    # curl exits non-zero on connection refused; treat empty as 000
    code="${code:-000}"
    echo "$code"
}

check() {
    local name="$1"
    local expected="$2"
    local code="$3"
    if [ "$code" = "$expected" ]; then
        printf "  %-45s [%s]  OK\n" "==> $name" "$code"
        PASS=$((PASS + 1))
    else
        printf "  %-45s [%s]  FAIL (expected %s)\n" "==> $name" "$code" "$expected"
        FAIL=$((FAIL + 1))
    fi
}

# ─── fdb-gateway checks ───────────────────────────────────────────────────────

echo ""
echo "── fdb-gateway  ($BASE) ──────────────────────────────────────"

code=$(http_code "/healthz (unauthenticated)" "$BASE/healthz")
check "/healthz" "200" "$code"

code=$(http_code "/openapi.json" "$BASE/openapi.json")
check "/openapi.json" "200" "$code"

code=$(http_code "/metrics (Prometheus scrape)" "$BASE/metrics")
check "/metrics" "200" "$code"

if [ -n "$TOKEN" ]; then
    code=$(http_code "/a2ui/v1/components (auth)" "$BASE/a2ui/v1/components" \
           -H "Authorization: Bearer $TOKEN")
    check "/a2ui/v1/components" "200" "$code"

    code=$(http_code "/mcp/v1/tools (auth)" "$BASE/mcp/v1/tools" \
           -H "Authorization: Bearer $TOKEN")
    check "/mcp/v1/tools" "200" "$code"
else
    echo "  ==> /a2ui/v1/components         SKIP (SMOKE_TOKEN not set)"
    echo "  ==> /mcp/v1/tools               SKIP (SMOKE_TOKEN not set)"
fi

# 401 expected when no token is supplied — confirms auth middleware is active.
code=$(http_code "/a2ui/v1/components (no auth → 401)" "$BASE/a2ui/v1/components")
check "/a2ui/v1/components (no-auth guard)" "401" "$code"

# ─── fke-server checks ───────────────────────────────────────────────────────

echo ""
echo "── fke-server  ($KILN) ──────────────────────────────────────"

code=$(http_code "/healthz" "$KILN/healthz")
check "/healthz" "200" "$code"

# 404/405 expected for an unknown function name — confirms the invoke route is
# wired and Cedar gate is active (rather than the process not responding at all).
code=$(http_code "POST /functions/v1/smoke-nonexistent (404)" \
       "$KILN/functions/v1/__smoke_nonexistent__" -X POST)
check "invoke unknown function returns 4xx" "404" "$code" || true
# tolerate 401/403/404 — any 4xx means the server is alive and gating correctly
if [ "$code" != "404" ] && [ "$code" != "401" ] && [ "$code" != "403" ]; then
    printf "  WARNING: invoke returned %s; expected 4xx\n" "$code"
fi

# ─── summary ─────────────────────────────────────────────────────────────────

echo ""
echo "──────────────────────────────────────────────────────────────"
printf "  Passed: %d   Failed: %d\n" "$PASS" "$FAIL"
echo "──────────────────────────────────────────────────────────────"

if [ "$FAIL" -gt 0 ]; then
    echo "SMOKE TESTS FAILED"
    exit 1
fi

echo "All smoke tests passed ✓"
