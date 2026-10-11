-- Full-image first-boot extension creation.
CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_net;

-- Anvil pgrx extensions (all built from source in the anvil stage).
-- Order matters: auth creates shared JWT roles; vault must precede meta
-- because meta requires vault_admin and declares requires = 'flint_vault'.
CREATE EXTENSION IF NOT EXISTS "ext-flint-auth";   -- auth.* identity helpers + roles
CREATE EXTENSION IF NOT EXISTS "flint_vault";      -- encrypted secret store
CREATE EXTENSION IF NOT EXISTS "ext-flint-meta";   -- flint_meta schema cache
CREATE EXTENSION IF NOT EXISTS "ext-flint-hooks";  -- flint.webhooks / outbox
CREATE EXTENSION IF NOT EXISTS "flint_llm";        -- LLM async job queue

-- pg_graphql: installed from the pinned v1.6.2 PG18 release .deb (see the Dockerfile).
-- NOT tolerated when absent. The previous warning-only handler let the image boot green
-- with GraphQL silently missing, so a stack that could not serve a single GraphQL query
-- looked healthy. GraphQL is the client sync path, not an optional passthrough: if the
-- extension is not there, first boot must fail here rather than at a customer's query.
CREATE EXTENSION IF NOT EXISTS pg_graphql;

-- pg_cron: scheduled jobs
CREATE EXTENSION IF NOT EXISTS pg_cron;

-- Prime the flint_meta schema cache so reflection sees columns for all tables
-- (including extension tables such as cron.job) from the first boot.
SELECT flint_meta.full_refresh();

-- Webhook outbox GC: delete processed/failed entries older than 7 days
SELECT cron.schedule('webhook-outbox-gc', '0 3 * * *',
  $$DELETE FROM flint.webhook_outbox WHERE status IN ('delivered', 'failed') AND updated_at < now() - interval '7 days'$$);

-- Meta full-refresh: nightly schema cache rebuild (function defined in ext-flint-meta).
SELECT cron.schedule('meta-full-refresh', '0 2 * * *',
  $$SELECT flint_meta.full_refresh()$$);

-- Durable webhook dispatcher: process outbox every minute.
SELECT cron.schedule('webhook-outbox-processor', '* * * * *',
  $$SELECT flint.process_webhook_outbox()$$);
