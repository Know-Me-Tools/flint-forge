#!/usr/bin/env bash
# scripts/restore_pg_pitr.sh — point-in-time restore drill/procedure for the
# production Postgres data plane, using wal-g base backups + continuously
# archived WAL.
#
# THIS SCRIPT MUST BE RUN BY AN OPERATOR, NOT AUTOMATICALLY. It stops the
# running `db` container, replaces its data directory with a restored backup,
# and starts Postgres in recovery mode. That is exactly the class of
# hard-to-reverse, production-affecting action that requires a human decision
# per change and confirmation at each step — see docs/runbook.md §13.4.3 for
# the full walkthrough this script automates. Task p16-c008 requires this
# drill be executed and documented at least once before the "staging only"
# caveat on the `db` service in docker-compose.prod.yml can be considered
# resolved.
#
# Usage:
#   ./scripts/restore_pg_pitr.sh [--target-time '2026-07-14 03:00:00+00' | --latest]
#
# What it does:
#   1. Confirms with the operator before touching anything (unless --yes).
#   2. Stops the `db` container (app containers will see connection errors —
#      expected; this IS an outage drill) and the `backup` sidecar (it holds
#      a read-only mount of the same volume and must not race the restore).
#   3. Runs `wal-g backup-fetch` via a one-off `db`-image container with the
#      volume mounted read-write (the `backup` service mounts it read-only,
#      so it cannot be used to write a restored backup into place).
#   4. Writes recovery.signal + restore_command/recovery_target_time so
#      Postgres replays WAL up to the requested point on next start.
#   5. Starts `db` and waits for recovery to complete.
#   6. Prints verification steps for the operator to confirm data integrity.
set -euo pipefail

TARGET_TIME=""
ASSUME_YES=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --target-time)
            TARGET_TIME="$2"; shift 2 ;;
        --latest)
            TARGET_TIME=""; shift ;;
        --yes)
            ASSUME_YES=true; shift ;;
        -h|--help)
            grep "^#" "$0" | sed 's/^# \?//'; exit 0 ;;
        *)
            echo "Unknown argument: $1" >&2; exit 1 ;;
    esac
done

COMPOSE="docker compose -f docker-compose.yml -f docker-compose.prod.yml"

echo "=== Flint Forge — Postgres PITR restore drill ==="
echo "Target: ${TARGET_TIME:-latest available backup}"
echo ""
echo "THIS WILL STOP THE PRODUCTION db CONTAINER. Application traffic will fail"
echo "until recovery completes. Only proceed during a planned drill or a real"
echo "incident."
echo ""

if ! "$ASSUME_YES"; then
    read -rp "Type 'restore' to continue: " CONFIRM
    if [[ "$CONFIRM" != "restore" ]]; then
        echo "Aborted." >&2
        exit 1
    fi
fi

echo "--- Step 1/5: stopping db and backup ---"
$COMPOSE stop backup db

echo "--- Step 2/5: fetching base backup via wal-g ---"
echo "NOTE: this restores into the SAME postgres_data volume the db service"
echo "uses. Ensure you have a separate snapshot of the current volume first"
echo "if you need to preserve pre-restore state for forensics."
echo "Using the db service's own image/entrypoint (RW volume access; the"
echo "backup sidecar's mount is read-only and cannot write a restore here)."
$COMPOSE run --rm --no-deps --entrypoint bash db -c '
    set -e
    if [ -r /run/secrets/walg_s3_access_key ]; then
        export AWS_ACCESS_KEY_ID; AWS_ACCESS_KEY_ID=$(cat /run/secrets/walg_s3_access_key)
    fi
    if [ -r /run/secrets/walg_s3_secret_key ]; then
        export AWS_SECRET_ACCESS_KEY; AWS_SECRET_ACCESS_KEY=$(cat /run/secrets/walg_s3_secret_key)
    fi
    wal-g backup-fetch /var/lib/postgresql/data LATEST
'

echo "--- Step 3/5: configuring recovery target ---"
RECOVERY_CONF="restore_command = 'wal-g wal-fetch %f %p'
recovery_target_action = 'promote'"
if [[ -n "$TARGET_TIME" ]]; then
    RECOVERY_CONF="${RECOVERY_CONF}
recovery_target_time = '${TARGET_TIME}'"
fi
$COMPOSE run --rm --no-deps --entrypoint bash db -c "
    set -e
    touch /var/lib/postgresql/data/recovery.signal
    cat >> /var/lib/postgresql/data/postgresql.auto.conf <<'RECOVERY_EOF'
${RECOVERY_CONF}
RECOVERY_EOF
"

echo "--- Step 4/5: starting db in recovery mode ---"
$COMPOSE up -d db

echo "--- Step 5/5: waiting for recovery to complete ---"
for i in $(seq 1 60); do
    if $COMPOSE exec -T db pg_isready -U flint -d flint > /dev/null 2>&1; then
        echo "db is accepting connections (recovery likely complete)."
        break
    fi
    echo "  waiting for recovery ($i/60)..."
    sleep 5
done

cat <<'EOF'

=== Verify the restore before declaring the drill successful ===

1. Confirm Postgres left recovery mode:
     docker compose -f docker-compose.yml -f docker-compose.prod.yml \
       exec db psql -U flint -d flint -c "SELECT pg_is_in_recovery();"
   Expect: f (false) — recovery_target_action=promote exits recovery once the
   target is reached.

2. Spot-check row counts / a known recent row against expectations for the
   target time you restored to.

3. Record the drill result (date, target time, verification query output,
   any anomalies) in docs/runbook.md §13.4.3 — this is the evidence required
   to close out the p16-c008 restore-drill task.

4. If anything looks wrong, STOP — do not resume production traffic against
   this data directory. Restore the pre-drill volume snapshot instead.

5. Once verified, restart the backup sidecar (stopped in step 1):
     docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d backup
EOF
