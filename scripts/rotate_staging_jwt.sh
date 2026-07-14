#!/usr/bin/env bash
# scripts/rotate_staging_jwt.sh — Rotate the `staging` Environment's JWT_SECRET.
#
# Usage:
#   ./scripts/rotate_staging_jwt.sh              # generate + upload new secret
#   ./scripts/rotate_staging_jwt.sh --dry-run    # show what would happen without writing
#
# What it does:
#   1. Generates a fresh 32-byte hex random JWT signing key.
#   2. Writes it locally to secrets/jwt_secret.txt (for mint_smoke_token.sh).
#   3. Uploads the value to GitHub as the `staging` Environment's `JWT_SECRET`
#      via `gh secret set JWT_SECRET --env staging` (p16-c008: renamed from the
#      repo-level `STAGING_JWT_SECRET` so `deploy.yml` can reuse the same
#      generic secret names per-Environment for both staging and production —
#      see docs/runbook.md §9.1/§13).
#
# Prerequisites:
#   - `gh` CLI installed and authenticated.
#   - Push access to the repository so the Actions secret can be updated.
#
# After running, the next GitHub Actions workflow that uses the `staging`
# Environment's JWT_SECRET will pick up the new value automatically. Existing
# JWTs signed with the old secret will fail validation once the gateway
# restarts with the new key.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SECRETS_DIR="${ROOT}/secrets"
DRY_RUN=false

# ── parse args ────────────────────────────────────────────────────────────────
for arg in "$@"; do
    case "$arg" in
        --dry-run) DRY_RUN=true ;;
        -h|--help)
            grep "^#" "$0" | sed 's/^# \?//'
            exit 0
            ;;
        *) echo "Unknown argument: $arg" >&2; exit 1 ;;
    esac
done

# ── helpers ───────────────────────────────────────────────────────────────────
write_secret() {
    local file="$1"
    local value="$2"
    if "$DRY_RUN"; then
        echo "  [dry-run] would write: ${file}"
    else
        printf '%s' "$value" > "$file"
        chmod 600 "$file"
        echo "  written: ${file}"
    fi
}

# ── generate secret ───────────────────────────────────────────────────────────
JWT_SECRET=$(openssl rand -hex 32)

# ── write local file ──────────────────────────────────────────────────────────
if ! "$DRY_RUN"; then
    mkdir -p "$SECRETS_DIR"
    chmod 700 "$SECRETS_DIR"
fi

echo "Rotating staging JWT secret..."
echo ""
write_secret "${SECRETS_DIR}/jwt_secret.txt" "$JWT_SECRET"

# ── update GitHub Actions secret ──────────────────────────────────────────────
echo ""
echo "Updating the 'staging' Environment's JWT_SECRET..."
if "$DRY_RUN"; then
    echo "  [dry-run] would run: gh secret set JWT_SECRET --env staging --body '<redacted>'"
    echo "  [dry-run] would restart the staging stack to load the new secret"
else
    if ! command -v gh >/dev/null 2>&1; then
        echo "  ERROR: gh CLI not found. Install it from https://cli.github.com" >&2
        exit 1
    fi
    printf '%s' "$JWT_SECRET" | gh secret set JWT_SECRET --env staging --body "$JWT_SECRET"
    echo "  uploaded: JWT_SECRET (staging Environment)"
fi

# ── summary ───────────────────────────────────────────────────────────────────
echo ""
if "$DRY_RUN"; then
    echo "Done (dry run). No files or secrets were changed."
else
    echo "Done. The 'staging' Environment's JWT_SECRET was rotated."
fi
echo ""
echo "To apply the new secret, restart the staging stack:"
echo ""
echo "  docker compose -f docker-compose.yml -f docker-compose.staging.yml up -d"
echo ""
echo "Then mint a fresh smoke token and run smoke tests:"
echo ""
echo "  TOKEN=\$(./scripts/mint_smoke_token.sh)"
echo "  BASE_URL=https://forge.example.com KILN_URL=http://localhost:8090 \\"
echo "    SMOKE_TOKEN=\$TOKEN ./scripts/smoke_test.sh"
