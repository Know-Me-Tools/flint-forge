# p1-c004 — pg_cron: add to Postgres 18 Docker image

## Why

The durable webhook BGW needs pg_cron for GC job scheduling. The `flint_meta` refresh pipeline also benefits from a scheduled full-refresh as a safety net for the `CREATE TABLE AS` DDL gap. pg_cron must be available in the target container before any GC job can be registered.

## What

- Add `pg_cron` to `images/postgres18/Dockerfile`: install the `postgresql-17-cron` → `postgresql-18-cron` package (or build from source if PG18 package not yet available in the target APT repo)
- Add `pg_cron` to `shared_preload_libraries` in the container's `postgresql.conf` (or via `ALTER SYSTEM`)
- Add `pg_cron` to `CREATE EXTENSION IF NOT EXISTS pg_cron` in the Forge init SQL
- Register the webhook outbox GC job: `SELECT cron.schedule('webhook-outbox-gc', '0 3 * * *', $$DELETE FROM flint.webhook_outbox WHERE status = 'delivered' AND created_at < now() - INTERVAL '7 days'$$)`
- Register the flint_meta full-refresh safety job: `SELECT cron.schedule('meta-full-refresh', '*/10 * * * *', $$SELECT flint_meta.full_refresh()$$)` (function implemented in p1-c009)

## Contract

`SELECT * FROM cron.job` returns at least two rows: `webhook-outbox-gc` and `meta-full-refresh`. `pg_cron` extension is loaded without errors on Postgres 18 container startup.

## Out of scope

Custom pg_cron schedules per-tenant (Phase 7+). The actual GC logic lives in the extension SQL.

## Constraints

- Do not modify `shared_preload_libraries` in a way that prevents the container from starting if pg_cron is absent — use `dynamic_library_path` or fail gracefully
- pg_cron requires `cron.database_name` GUC set to the target database name

## Reference

- `images/postgres18/Dockerfile` (target file)
- pg_cron README: https://github.com/citusdata/pg_cron
- RFC-FORGE-001 §8 (container tooling)
