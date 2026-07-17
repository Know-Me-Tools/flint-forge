#!/bin/bash
# images/postgres18/entrypoint-walg.sh — wraps the official postgres entrypoint
# to load wal-g's S3 credentials from Docker secrets before starting Postgres.
#
# p16-c008: this AWS/S3-credential-file translation is for the docker-compose
# path ONLY (docker-compose.prod.yml) — not our supported production target.
# The Kubernetes path (deploy/helm/flint-forge) never sets these secret
# files; it authenticates wal-g to Azure Blob Storage entirely via Azure AD
# Workload Identity (see postgres.yaml, docs/runbook.md §13) — no credential
# files or translation needed there, so this script is a pure no-op
# passthrough for every Kubernetes pod.
#
# Fallback: if the secret files are absent (Kubernetes, local dev, CI), this
# is a no-op passthrough to the original entrypoint.
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
