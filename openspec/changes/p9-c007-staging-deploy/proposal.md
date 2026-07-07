# p9-c007 — Staging Deploy (Smoke Tests + Deploy Workflow)

**Phase:** 9 — Production Hardening
**Priority:** P2
**Depends on:** p9-c001 (docker-compose.staging.yml extends p9-c001 compose)

## What this change delivers

- `docker-compose.staging.yml` — production-safe compose for staging host
- `scripts/smoke_test.sh` — automated health + functional checks after deploy
- `.github/workflows/deploy.yml` — manual-trigger deploy workflow

## Design

### `scripts/smoke_test.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail
BASE="${BASE_URL:-http://localhost:8080}"
TOKEN="${SMOKE_TOKEN:-}"

check() { curl -sf -o /dev/null -w "%{http_code}" "$@"; }

echo "==> /healthz"
[ "$(check "$BASE/healthz")" = "200" ] || { echo "FAIL: /healthz"; exit 1; }

echo "==> /a2ui/v1/components (authenticated)"
[ "$(check -H "Authorization: Bearer $TOKEN" "$BASE/a2ui/v1/components")" = "200" ] \
  || { echo "FAIL: /a2ui/v1/components"; exit 1; }

echo "==> /mcp/v1/tools (authenticated)"
[ "$(check -H "Authorization: Bearer $TOKEN" "$BASE/mcp/v1/tools")" = "200" ] \
  || { echo "FAIL: /mcp/v1/tools"; exit 1; }

echo "==> fke-server /healthz"
[ "$(check "${KILN_URL:-http://localhost:8090}/healthz")" = "200" ] \
  || { echo "FAIL: fke-server /healthz"; exit 1; }

echo "All smoke tests passed ✓"
```

### `.github/workflows/deploy.yml`

Manual trigger on `workflow_dispatch` with `environment` input:

```yaml
on:
  workflow_dispatch:
    inputs:
      environment:
        description: 'Target environment (staging)'
        default: staging
```

Steps: SSH to staging host → `docker compose pull` → `docker compose up -d` → run `smoke_test.sh`.

### `docker-compose.staging.yml`

```yaml
version: '3.9'
services:
  db:
    restart: unless-stopped
    deploy:
      resources:
        limits: { cpus: '1', memory: 1G }
  fdb-gateway:
    restart: unless-stopped
    deploy:
      resources:
        limits: { cpus: '2', memory: 2G }
  fke-server:
    restart: unless-stopped
    deploy:
      resources:
        limits: { cpus: '1', memory: 1G }
```
