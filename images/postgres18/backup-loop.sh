#!/bin/bash
# images/postgres18/backup-loop.sh — periodic wal-g base backups (p16-c008).
#
# Runs in the `backup` service (docker-compose.prod.yml), sharing the same
# pinned image as `db` (wal-g is already installed there). Loops forever:
# sleep WALG_BACKUP_INTERVAL_SECS, then `wal-g backup-push` against a
# read-only mount of the same data volume, connecting to `db` over the
# network for the START_BACKUP/STOP_BACKUP protocol.
#
# Fallback: if the S3 credential secret files are absent (local dev / CI /
# staging, where backups are not configured), this logs a warning once and
# exits 0 rather than looping forever doing nothing useful — a supervised
# `restart: unless-stopped` service that immediately exits clean is a much
# clearer signal to an operator than one that silently spins.
set -euo pipefail

if [ ! -r /run/secrets/walg_s3_access_key ] || [ ! -r /run/secrets/walg_s3_secret_key ]; then
    echo "backup-loop: wal-g S3 credentials not provisioned (see docs/runbook.md §13.4) — exiting, not backing up." >&2
    exit 0
fi

export AWS_ACCESS_KEY_ID
AWS_ACCESS_KEY_ID=$(cat /run/secrets/walg_s3_access_key)
export AWS_SECRET_ACCESS_KEY
AWS_SECRET_ACCESS_KEY=$(cat /run/secrets/walg_s3_secret_key)

if [ -r /run/secrets/postgres_password ]; then
    export PGPASSWORD
    PGPASSWORD=$(cat /run/secrets/postgres_password)
fi

INTERVAL="${WALG_BACKUP_INTERVAL_SECS:-86400}"

echo "backup-loop: starting, interval=${INTERVAL}s, target=${WALG_S3_PREFIX:-<unset>}"

while true; do
    echo "backup-loop: $(date -u +%Y-%m-%dT%H:%M:%SZ) running wal-g backup-push"
    backup_status=0
    wal-g backup-push /var/lib/postgresql/data || backup_status=$?
    if [ "$backup_status" -eq 0 ]; then
        echo "backup-loop: $(date -u +%Y-%m-%dT%H:%M:%SZ) backup-push succeeded"
    else
        echo "backup-loop: $(date -u +%Y-%m-%dT%H:%M:%SZ) backup-push FAILED (exit ${backup_status})" >&2
    fi
    sleep "${INTERVAL}"
done
