#!/bin/bash
# images/postgres18/entrypoint-walg.sh — wraps the official postgres entrypoint
# to load wal-g's S3 credentials from Docker secrets before starting Postgres.
#
# p16-c008: continuous WAL archiving (archive_command=wal-g wal-push) runs
# inside this same container, so wal-g needs AWS_ACCESS_KEY_ID/
# AWS_SECRET_ACCESS_KEY in its environment at the time archive_command fires.
# Docker secrets are mounted as files, not env vars, so we translate them here
# — the same pattern used by docker/fdb-gateway/entrypoint.sh and
# docker/fke-server/entrypoint.sh for their own secrets.
#
# Fallback: if the secret files are absent (local dev / CI / staging, where
# wal-g archiving is not configured), this is a no-op passthrough to the
# original entrypoint.
set -e

if [ -r /run/secrets/walg_s3_access_key ]; then
    export AWS_ACCESS_KEY_ID
    AWS_ACCESS_KEY_ID=$(cat /run/secrets/walg_s3_access_key)
fi

if [ -r /run/secrets/walg_s3_secret_key ]; then
    export AWS_SECRET_ACCESS_KEY
    AWS_SECRET_ACCESS_KEY=$(cat /run/secrets/walg_s3_secret_key)
fi

exec docker-entrypoint.sh "$@"
