# p11-c006 — Staging Token Rotation

**Phase:** 11 — API Stability  **Priority:** P2  **Depends on:** none

## Problem

`STAGING_SMOKE_TOKEN` in `.github/workflows/deploy.yml` is a static long-lived
JWT stored as a repository secret. Long-lived tokens are a security risk if the
secret is exposed. The `deploy.yml` should mint a fresh 1-hour token before
each smoke test run rather than relying on a static value.

## Solution

`forge-identity` verifies JWTs but does not issue them. `jsonwebtoken` is in
the workspace. A shell script using `openssl` and a compact Python/Node one-liner
can produce a self-signed HS256 JWT without adding new build dependencies.

### `scripts/mint_smoke_token.sh`

```bash
#!/usr/bin/env bash
# Mint a short-lived HS256 JWT for smoke testing.
#
# Usage:
#   ./scripts/mint_smoke_token.sh > /tmp/smoke_token
#   TOKEN=$(./scripts/mint_smoke_token.sh) k6 run regression.js
#
# Reads signing key from:
#   1. $JWT_SECRET env var
#   2. secrets/jwt_secret.txt (local dev / staging host)
#   3. /run/secrets/jwt_secret (inside container)
set -euo pipefail

if   [ -n "${JWT_SECRET:-}" ];               then SECRET="$JWT_SECRET"
elif [ -r "secrets/jwt_secret.txt" ];        then SECRET=$(cat secrets/jwt_secret.txt)
elif [ -r "/run/secrets/jwt_secret" ];       then SECRET=$(cat /run/secrets/jwt_secret)
else echo "No JWT secret found" >&2; exit 1; fi

# Build a minimal claims set: sub=smoke, role=authenticated, exp=now+3600
NOW=$(date +%s)
EXP=$((NOW + 3600))
HEADER=$(printf '{"alg":"HS256","typ":"JWT"}' | base64 -w0 | tr '+/' '-_' | tr -d '=')
PAYLOAD=$(printf '{"sub":"smoke","role":"authenticated","exp":%d,"iat":%d}' \
  "$EXP" "$NOW" | base64 -w0 | tr '+/' '-_' | tr -d '=')
SIG=$(printf '%s.%s' "$HEADER" "$PAYLOAD" | \
  openssl dgst -sha256 -hmac "$SECRET" -binary | \
  base64 -w0 | tr '+/' '-_' | tr -d '=')
printf '%s.%s.%s\n' "$HEADER" "$PAYLOAD" "$SIG"
```

### `.github/workflows/deploy.yml` update

Replace the static `STAGING_SMOKE_TOKEN` usage with a mint step:

```yaml
      - name: Mint smoke token
        run: |
          chmod +x scripts/mint_smoke_token.sh
          SMOKE_TOKEN=$(JWT_SECRET="${{ secrets.STAGING_JWT_SECRET }}" \
            ./scripts/mint_smoke_token.sh)
          echo "SMOKE_TOKEN=${SMOKE_TOKEN}" >> "$GITHUB_ENV"

      - name: Run smoke tests
        env:
          BASE_URL: http://localhost:8080
          KILN_URL: http://localhost:8090
        run: |
          chmod +x smoke_test.sh
          SMOKE_TOKEN="${SMOKE_TOKEN}" ./smoke_test.sh
```

Note: `STAGING_JWT_SECRET` replaces `STAGING_SMOKE_TOKEN` as the repository secret.
It contains the raw signing key (content of `secrets/jwt_secret.txt`), not a token.

### Runbook §11

Document the `mint_smoke_token.sh` usage and the `STAGING_JWT_SECRET` secret.
