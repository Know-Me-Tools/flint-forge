#!/usr/bin/env bash
# backup_restore.sh — Regression test for extension-config-table dump/restore.
#
# Guards against a P0 data-loss bug: Postgres excludes extension-owned tables
# (pg_depend deptype='e') from pg_dump by default unless the extension calls
# pg_extension_config_dump() on them during install. Without that call, a
# standard `pg_dump | pg_restore` backup silently drops every row in
# vault.secrets and vault.access_log on restore — not just becomes-unreadable
# without the DEK, the rows vanish entirely.
#
# This test creates real rows, dumps the database, restores into a second
# database on the same cluster, and asserts row counts match. It requires a
# running Postgres 18 instance with flint_vault installed (matching
# `cargo pgrx test` / images/postgres18/Dockerfile.vault-check environments)
# and exercises pg_dump/pg_restore as separate processes — something a
# `#[pg_test]` SPI-based test cannot do, since that runs inside the single
# ephemeral test Postgres process.
#
# Usage:
#   PGHOST=... PGPORT=... PGUSER=... ./backup_restore.sh
#
# Exits non-zero on any row-count mismatch or missing table.

set -euo pipefail

PGHOST="${PGHOST:-localhost}"
PGPORT="${PGPORT:-28818}"
PGUSER="${PGUSER:-postgres}"
SRC_DB="flint_vault_backup_test_src"
DST_DB="flint_vault_backup_test_dst"
DUMP_FILE="$(mktemp -u /tmp/flint_vault_backup_test_XXXXXX.dump)"

export PGHOST PGPORT PGUSER

psql_src() { psql -X -q -v ON_ERROR_STOP=1 -d "$SRC_DB" "$@"; }
psql_dst() { psql -X -q -v ON_ERROR_STOP=1 -d "$DST_DB" "$@"; }

cleanup() {
    rm -f "$DUMP_FILE"
    psql -X -q -d postgres -c "DROP DATABASE IF EXISTS $SRC_DB;" 2>/dev/null || true
    psql -X -q -d postgres -c "DROP DATABASE IF EXISTS $DST_DB;" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

echo "==> Creating source database and installing flint_vault..." >&2
psql -X -q -d postgres -c "DROP DATABASE IF EXISTS $SRC_DB;"
psql -X -q -d postgres -c "CREATE DATABASE $SRC_DB;"
psql_src -c "CREATE EXTENSION IF NOT EXISTS pgcrypto;"
psql_src -c "CREATE EXTENSION flint_vault;"

echo "==> Seeding secrets and access-log rows..." >&2
psql_src -c "SELECT vault.create_secret('password','pg-prod-pw-9!','backup-test-secret','backup/restore regression test',NULL,NULL);"
psql_src -c "SELECT vault.create_secret('api_key','sk-test-backup-1','backup-test-key','',  'anthropic','test-scope');"

secrets_before=$(psql_src -X -t -A -c "SELECT count(*) FROM vault.secrets;")
log_before=$(psql_src -X -t -A -c "SELECT count(*) FROM vault.access_log;")

if [[ "$secrets_before" -lt 2 ]]; then
    echo "FAIL: expected >=2 seeded rows in vault.secrets, got $secrets_before" >&2
    exit 1
fi
if [[ "$log_before" -lt 2 ]]; then
    echo "FAIL: expected >=2 seeded rows in vault.access_log, got $log_before" >&2
    exit 1
fi
echo "    vault.secrets=$secrets_before vault.access_log=$log_before (before dump)" >&2

echo "==> Running pg_dump -Fc..." >&2
pg_dump -Fc -d "$SRC_DB" -f "$DUMP_FILE"

echo "==> Verifying pg_restore -l lists vault.secrets / vault.access_log TOC entries..." >&2
toc=$(pg_restore -l "$DUMP_FILE")
if ! grep -q "TABLE DATA.*vault secrets" <<<"$toc"; then
    echo "FAIL: pg_dump TOC has no TABLE DATA entry for vault.secrets — rows would be silently dropped on restore" >&2
    exit 1
fi
if ! grep -q "TABLE DATA.*vault access_log" <<<"$toc"; then
    echo "FAIL: pg_dump TOC has no TABLE DATA entry for vault.access_log — rows would be silently dropped on restore" >&2
    exit 1
fi

echo "==> Restoring into a fresh database..." >&2
psql -X -q -d postgres -c "DROP DATABASE IF EXISTS $DST_DB;"
psql -X -q -d postgres -c "CREATE DATABASE $DST_DB;"
psql_dst -c "CREATE EXTENSION IF NOT EXISTS pgcrypto;"
pg_restore -d "$DST_DB" "$DUMP_FILE"

secrets_after=$(psql_dst -X -t -A -c "SELECT count(*) FROM vault.secrets;")
log_after=$(psql_dst -X -t -A -c "SELECT count(*) FROM vault.access_log;")

echo "    vault.secrets=$secrets_after vault.access_log=$log_after (after restore)" >&2

status=0
if [[ "$secrets_after" != "$secrets_before" ]]; then
    echo "FAIL: vault.secrets row count mismatch: before=$secrets_before after=$secrets_after" >&2
    status=1
fi
if [[ "$log_after" != "$log_before" ]]; then
    echo "FAIL: vault.access_log row count mismatch: before=$log_before after=$log_after" >&2
    status=1
fi

if [[ "$status" -eq 0 ]]; then
    echo "OK: pg_dump/pg_restore round-trip preserved all vault.secrets and vault.access_log rows" >&2
fi
exit "$status"
