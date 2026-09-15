\set ON_ERROR_STOP on
-- Run explicitly on existing databases after the pg_net/pg_cron image lands.
-- Wires durable webhook delivery: flint.process_webhook_outbox uses
-- net.http_post (pg_net) and is driven every minute by pg_cron.
-- Idempotent; safe to re-run. Requires shared_preload_libraries to already
-- include pg_net,pg_cron and cron.database_name=flint (a restart is needed
-- when those settings change — run this after the rollout).
BEGIN;
SELECT pg_advisory_xact_lock(721405002);
CREATE EXTENSION IF NOT EXISTS pg_net;
CREATE EXTENSION IF NOT EXISTS pg_cron;
COMMIT;

-- cron.schedule upserts by job name, so these are idempotent.

-- Durable webhook dispatcher: process outbox every minute.
SELECT cron.schedule('webhook-outbox-processor', '* * * * *',
  $$SELECT flint.process_webhook_outbox()$$);

-- Webhook outbox GC: delete processed/failed entries older than 7 days.
SELECT cron.schedule('webhook-outbox-gc', '0 3 * * *',
  $$DELETE FROM flint.webhook_outbox WHERE status IN ('delivered', 'failed') AND updated_at < now() - interval '7 days'$$);

-- Meta full-refresh: nightly schema cache rebuild.
SELECT cron.schedule('meta-full-refresh', '0 2 * * *',
  $$SELECT flint_meta.full_refresh()$$);
