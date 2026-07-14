#!/usr/bin/env bash
# scripts/rotate_secrets.sh — Generate Docker secret files for Flint Forge production.
#
# Usage:
#   ./scripts/rotate_secrets.sh              # generate all secrets
#   ./scripts/rotate_secrets.sh --dry-run    # show what would be written without writing
#
# What it creates (in secrets/):
#   jwt_secret.txt              — 32-byte hex random string for JWT signing
#   postgres_password.txt       — 16-byte hex random string for PostgreSQL
#   caddy_tls_email.txt         — reads CADDY_TLS_EMAIL from env (or prompts if unset)
#   walg_s3_access_key.txt      — reads WALG_S3_ACCESS_KEY from env (optional — see below)
#   walg_s3_secret_key.txt      — reads WALG_S3_SECRET_KEY from env (optional — see below)
#
# The wal-g secrets are OPTIONAL (unlike the three above, which are always
# written): if WALG_S3_ACCESS_KEY/WALG_S3_SECRET_KEY aren't set in the
# environment, this script skips them entirely rather than prompting — an
# operator who hasn't provisioned backup storage yet can still rotate the
# other secrets. `db`'s entrypoint and the `backup` service both no-op
# cleanly without these files (docs/runbook.md §13.4).
#
# After rotation, restart the affected services:
#   docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d db fdb-gateway fke-server caddy
#
# Security:
#   - Secret files are 0600 (owner-read-only).
#   - The secrets/ directory is 0700.
#   - secrets/ is in .gitignore — NEVER commit these files.
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

# ── create secrets dir ────────────────────────────────────────────────────────
if ! "$DRY_RUN"; then
    mkdir -p "$SECRETS_DIR"
    chmod 700 "$SECRETS_DIR"
fi

echo "Rotating Flint Forge secrets..."
echo ""

# ── jwt_secret ────────────────────────────────────────────────────────────────
echo "JWT signing key (jwt_secret.txt):"
JWT_SECRET=$(openssl rand -hex 32)
write_secret "${SECRETS_DIR}/jwt_secret.txt" "$JWT_SECRET"

# ── postgres_password ─────────────────────────────────────────────────────────
echo "PostgreSQL password (postgres_password.txt):"
PG_PASSWORD=$(openssl rand -hex 16)
write_secret "${SECRETS_DIR}/postgres_password.txt" "$PG_PASSWORD"

# ── caddy_tls_email ───────────────────────────────────────────────────────────
echo "ACME TLS email (caddy_tls_email.txt):"
if [[ -n "${CADDY_TLS_EMAIL:-}" ]]; then
    TLS_EMAIL="${CADDY_TLS_EMAIL}"
else
    if [[ -t 0 ]]; then
        read -rp "  Enter ACME/Let's Encrypt email address: " TLS_EMAIL
    else
        echo "  CADDY_TLS_EMAIL not set and stdin is not a terminal." >&2
        echo "  Set CADDY_TLS_EMAIL before running this script, or run interactively." >&2
        exit 1
    fi
fi
write_secret "${SECRETS_DIR}/caddy_tls_email.txt" "$TLS_EMAIL"

# ── wal-g S3 credentials (optional — p16-c008) ────────────────────────────────
echo ""
if [[ -n "${WALG_S3_ACCESS_KEY:-}" && -n "${WALG_S3_SECRET_KEY:-}" ]]; then
    echo "wal-g S3 credentials (walg_s3_access_key.txt / walg_s3_secret_key.txt):"
    write_secret "${SECRETS_DIR}/walg_s3_access_key.txt" "$WALG_S3_ACCESS_KEY"
    write_secret "${SECRETS_DIR}/walg_s3_secret_key.txt" "$WALG_S3_SECRET_KEY"
else
    echo "wal-g S3 credentials: WALG_S3_ACCESS_KEY/WALG_S3_SECRET_KEY not set — skipping."
    echo "  (Backups stay disabled until these are provisioned — see docs/runbook.md §13.4.)"
fi

# ── .env update ───────────────────────────────────────────────────────────────
echo ""
echo "The following values must also be set in .env (gitignored) for the app"
echo "containers to connect to the database with the new password:"
echo ""
echo "  DATABASE_URL=postgres://flint:${PG_PASSWORD}@db:5432/flint"
echo ""

if ! "$DRY_RUN"; then
    ENV_FILE="${ROOT}/.env"
    if [[ -f "$ENV_FILE" ]]; then
        # Update or append DATABASE_URL
        if grep -q "^DATABASE_URL=" "$ENV_FILE"; then
            sed -i.bak "s|^DATABASE_URL=.*|DATABASE_URL=postgres://flint:${PG_PASSWORD}@db:5432/flint|" "$ENV_FILE"
            rm -f "${ENV_FILE}.bak"
            echo "  Updated DATABASE_URL in .env"
        else
            echo "DATABASE_URL=postgres://flint:${PG_PASSWORD}@db:5432/flint" >> "$ENV_FILE"
            echo "  Appended DATABASE_URL to .env"
        fi
    else
        echo "  .env not found — creating from .env.example and setting DATABASE_URL"
        cp "${ROOT}/.env.example" "$ENV_FILE"
        sed -i.bak "s|^DATABASE_URL=.*|DATABASE_URL=postgres://flint:${PG_PASSWORD}@db:5432/flint|" "$ENV_FILE"
        rm -f "${ENV_FILE}.bak"
    fi
fi

# ── summary ───────────────────────────────────────────────────────────────────
echo ""
echo "Done. Secret files written to: ${SECRETS_DIR}/"
echo ""
echo "To apply the new secrets, restart the affected services:"
echo ""
echo "  docker compose -f docker-compose.yml -f docker-compose.prod.yml \\"
echo "    up -d db fdb-gateway fke-server caddy"
echo ""
echo "NOTE: The postgres_password.txt secret is mounted at /run/secrets/postgres_password"
echo "  in the db container (POSTGRES_PASSWORD_FILE). The app containers read"
echo "  DATABASE_URL from .env which has been updated above."
