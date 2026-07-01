-- Full-image first-boot extension creation.
CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_net;
CREATE EXTENSION IF NOT EXISTS flint_auth;   -- SQL-only
CREATE EXTENSION IF NOT EXISTS flint_hooks;  -- SQL-only
CREATE EXTENSION IF NOT EXISTS flint_llm;    -- pgrx (Flint Ember)

-- pg_graphql: provisional on PG18 (built from master; no released PG18 build yet).
-- Tolerate absence so the image still boots; GraphQL passthrough degrades, not the data plane.
DO $$
BEGIN
  CREATE EXTENSION IF NOT EXISTS pg_graphql;
EXCEPTION WHEN others THEN
  RAISE WARNING 'pg_graphql unavailable (expected until supabase/pg_graphql v1.5.12 PG18 release): %', SQLERRM;
END $$;

-- pg_cron: scheduled jobs
CREATE EXTENSION IF NOT EXISTS pg_cron;

-- Webhook outbox GC: delete processed/failed entries older than 7 days
SELECT cron.schedule('webhook-outbox-gc', '0 3 * * *',
  $$DELETE FROM flint.webhook_outbox WHERE status IN ('delivered', 'failed') AND updated_at < now() - interval '7 days'$$
);

-- Meta full-refresh: nightly schema cache rebuild (function defined in p1-c009)
-- Registered as a stub; will call flint_meta.full_refresh() when p1-c009 ships.
SELECT cron.schedule('meta-full-refresh', '0 2 * * *',
  $$SELECT 1 -- placeholder: will be replaced with SELECT flint_meta.full_refresh() after p1-c009$$
);

-- Durable webhook dispatcher: process outbox every minute (added p1-c003).
SELECT cron.schedule('webhook-outbox-processor', '* * * * *',
  $$SELECT flint.process_webhook_outbox()$$
);
