#!/usr/bin/env bash
# scripts/mint_smoke_token.sh — Mint a short-lived HS256 JWT for smoke testing.
#
# Usage:
#   ./scripts/mint_smoke_token.sh              # prints JWT to stdout
#   TOKEN=$(./scripts/mint_smoke_token.sh)     # capture for use in scripts
#   JWT_SECRET=mysecret ./scripts/mint_smoke_token.sh   # explicit key
#
# The JWT has:
#   sub=smoke, role=authenticated, exp=now+3600, iat=now
#
# Signing key is read from (first match wins):
#   1. $JWT_SECRET env var
#   2. secrets/jwt_secret.txt
#   3. /run/secrets/jwt_secret
#
# Requires: openssl, base64, tr, date (all standard on macOS + Debian/Ubuntu)
set -euo pipefail

# ── Locate signing key ────────────────────────────────────────────────────────
if   [ -n "${JWT_SECRET:-}" ];               then SECRET="$JWT_SECRET"
elif [ -r "secrets/jwt_secret.txt" ];        then SECRET=$(cat secrets/jwt_secret.txt)
elif [ -r "/run/secrets/jwt_secret" ];       then SECRET=$(cat /run/secrets/jwt_secret)
else
    echo "ERROR: No JWT signing key found." >&2
    echo "  Set JWT_SECRET env var, or create secrets/jwt_secret.txt" >&2
    echo "  (run ./scripts/rotate_secrets.sh to generate secret files)" >&2
    exit 1
fi

# ── Build JWT ─────────────────────────────────────────────────────────────────
NOW=$(date +%s)
EXP=$((NOW + 3600))

# base64url encode — portable: use base64 then fix chars and strip padding/newlines
b64url() {
    printf '%s' "$1" | base64 | tr -d '\n' | tr '+/' '-_' | tr -d '='
}

HEADER=$(b64url '{"alg":"HS256","typ":"JWT"}')
PAYLOAD=$(b64url "{\"sub\":\"smoke\",\"role\":\"authenticated\",\"exp\":${EXP},\"iat\":${NOW}}")

# HMAC-SHA256 signature (binary, then base64url)
SIG=$(printf '%s.%s' "$HEADER" "$PAYLOAD" | \
    openssl dgst -sha256 -hmac "$SECRET" -binary | \
    base64 | tr -d '\n' | tr '+/' '-_' | tr -d '=')

printf '%s.%s.%s\n' "$HEADER" "$PAYLOAD" "$SIG"
