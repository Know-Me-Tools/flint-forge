\echo Use "CREATE EXTENSION flint_hooks" to load this file. \quit

CREATE SCHEMA IF NOT EXISTS flint;

CREATE TABLE IF NOT EXISTS flint.webhooks (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  schema_name text NOT NULL, table_name text NOT NULL,
  events text[] NOT NULL,
  target_url text NOT NULL,
  forward_jwt boolean NOT NULL DEFAULT false,
  custom_headers jsonb NOT NULL DEFAULT '{}',
  secret text NOT NULL,
  tier text NOT NULL DEFAULT 'standard',
  active boolean NOT NULL DEFAULT true,
  timeout_ms int NOT NULL DEFAULT 5000
);

CREATE TABLE IF NOT EXISTS flint.webhook_outbox (
  id bigserial PRIMARY KEY,
  webhook_id uuid NOT NULL,
  payload jsonb NOT NULL,
  headers jsonb NOT NULL,
  status text NOT NULL DEFAULT 'pending',
  visible_at timestamptz NOT NULL DEFAULT now(),
  retry_count int NOT NULL DEFAULT 0,
  created_at timestamptz NOT NULL DEFAULT now()
);

-- Generic SECURITY DEFINER dispatch trigger. Reads auth.bearer() in-transaction.
-- Option-3 default: Authorization: Bearer <service> + X-Forge-Origin-JWT + X-Forge-Signature.
-- standard tier -> net.http_post (pg_net);  durable tier -> insert into flint.webhook_outbox.
CREATE OR REPLACE FUNCTION flint.dispatch_webhook() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE payload jsonb;
BEGIN
  payload := jsonb_build_object(
    'type', TG_OP, 'schema', TG_TABLE_SCHEMA, 'table', TG_TABLE_NAME,
    'record', to_jsonb(NEW), 'old_record', to_jsonb(OLD));
  -- TODO(p1-c002/p1-c003): per-registration header build + tier routing.
  RETURN NEW;
END $$;
