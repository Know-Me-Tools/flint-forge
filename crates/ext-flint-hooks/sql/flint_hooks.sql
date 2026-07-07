CREATE SCHEMA IF NOT EXISTS flint;

CREATE TABLE IF NOT EXISTS flint.webhooks (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  schema_name text NOT NULL, table_name text NOT NULL,
  events text[] NOT NULL,
  target_url text,                          -- NULL when target_type != 'url'/'kiln'
  forward_jwt boolean NOT NULL DEFAULT false,
  custom_headers jsonb NOT NULL DEFAULT '{}',
  secret text NOT NULL,
  tier text NOT NULL DEFAULT 'standard' CHECK (tier IN ('standard', 'durable')),
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

-- Idempotent: add endpoint_url to webhook_outbox if not present (needed for durable tier).
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE  table_schema = 'flint'
          AND  table_name   = 'webhook_outbox'
          AND  column_name  = 'endpoint_url'
    ) THEN
        ALTER TABLE flint.webhook_outbox ADD COLUMN endpoint_url text;
    END IF;
END
$$;

-- p7-c001: Idempotent: add target_type column to flint.webhooks.
-- 'url'      — fire-and-forget HTTP POST (original behaviour, default)
-- 'agui_run' — emit AG-UI ToolCall events to /agents/v1/{agui_run_id}/events
-- 'kiln'     — stub for Phase 6 Kiln; queued as durable until Kiln BGW is live
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE  table_schema = 'flint'
          AND  table_name   = 'webhooks'
          AND  column_name  = 'target_type'
    ) THEN
        ALTER TABLE flint.webhooks
            ADD COLUMN target_type text NOT NULL DEFAULT 'url'
            CHECK (target_type IN ('url', 'agui_run', 'kiln'));
    END IF;
END
$$;

-- p7-c001: Idempotent: add agui_run_id column (target run for 'agui_run' targets).
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE  table_schema = 'flint'
          AND  table_name   = 'webhooks'
          AND  column_name  = 'agui_run_id'
    ) THEN
        ALTER TABLE flint.webhooks ADD COLUMN agui_run_id text;
    END IF;
END
$$;

-- p7-c002: Idempotent: add target_type to webhook_outbox (needed for agui_run BGW routing).
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE  table_schema = 'flint'
          AND  table_name   = 'webhook_outbox'
          AND  column_name  = 'target_type'
    ) THEN
        ALTER TABLE flint.webhook_outbox
            ADD COLUMN target_type text NOT NULL DEFAULT 'url';
    END IF;
END
$$;

-- p7-c002: Idempotent: add agui_run_id to webhook_outbox.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE  table_schema = 'flint'
          AND  table_name   = 'webhook_outbox'
          AND  column_name  = 'agui_run_id'
    ) THEN
        ALTER TABLE flint.webhook_outbox ADD COLUMN agui_run_id text;
    END IF;
END
$$;

-- pgcrypto is required for hmac() used in HMAC-SHA256 signing.
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Generic SECURITY DEFINER dispatch trigger.
-- standard tier: fire-and-forget HTTP POST via pg_net.
-- durable tier:  INSERT into flint.webhook_outbox for BGW pickup (p1-c003).
-- target_type routing:
--   'url'      — HTTP POST to target_url (original behaviour)
--   'agui_run' — Build AG-UI ToolCall events and POST to /agents/v1/{agui_run_id}/events
--                via pg_net; bypasses FRF agentproto (p7-c002 direct wire).
--   'kiln'     — Queue in webhook_outbox for future Kiln BGW (Phase 6 stub).
-- HMAC-SHA256 signature delivered as X-Forge-Signature: sha256=<hex>.
-- Caller JWT forwarding gated by wh.forward_jwt; value is NEVER logged.
CREATE OR REPLACE FUNCTION flint.dispatch_webhook()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = flint, net, public, pg_catalog
AS $$
DECLARE
    wh              record;
    payload         jsonb;
    payload_txt     text;
    sig_hex         text;
    headers         jsonb;
    bearer          text;
    agui_event      jsonb;
    agui_event_txt  text;
    agui_url        text;
    tool_call_id    text;
BEGIN
    -- Build the canonical event payload.
    payload := jsonb_build_object(
        'type',       TG_OP,
        'schema',     TG_TABLE_SCHEMA,
        'table',      TG_TABLE_NAME,
        'record',     CASE WHEN TG_OP = 'DELETE' THEN NULL ELSE to_jsonb(NEW) END,
        'old_record', CASE WHEN TG_OP = 'INSERT' THEN NULL ELSE to_jsonb(OLD) END,
        'timestamp',  now()
    );
    payload_txt := payload::text;

    FOR wh IN
        SELECT *
        FROM   flint.webhooks
        WHERE  schema_name = TG_TABLE_SCHEMA
          AND  table_name  = TG_TABLE_NAME
          AND  TG_OP       = ANY(events)
          AND  active      = true
    LOOP
        -- ── agui_run target (p7-c002 direct wire) ───────────────────────────
        -- Build AG-UI ToolCall events and POST to the run's event endpoint.
        -- The hook payload becomes a ToolCallResult with the table change data.
        IF wh.target_type = 'agui_run' AND wh.agui_run_id IS NOT NULL THEN
            tool_call_id := gen_random_uuid()::text;
            agui_url     := 'http://localhost:8080/agents/v1/'
                            || wh.agui_run_id || '/events';

            -- Emit ToolCallStart
            agui_event := jsonb_build_object(
                'event', jsonb_build_object(
                    'type',         'ToolCallStart',
                    'tool_call_id', tool_call_id,
                    'tool_name',    'hook:' || TG_TABLE_SCHEMA || '.' || TG_TABLE_NAME,
                    'parent_message_id', NULL
                )
            );
            PERFORM net.http_post(
                url     := agui_url,
                body    := agui_event::text::bytea,
                headers := '{"Content-Type":"application/json"}'::jsonb
            );

            -- Emit ToolCallResult with the full hook payload
            agui_event := jsonb_build_object(
                'event', jsonb_build_object(
                    'type',         'ToolCallResult',
                    'tool_call_id', tool_call_id,
                    'result',       payload,
                    'error',        NULL
                )
            );
            PERFORM net.http_post(
                url     := agui_url,
                body    := agui_event::text::bytea,
                headers := '{"Content-Type":"application/json"}'::jsonb
            );

            CONTINUE;
        END IF;

        -- ── kiln target stub (Phase 6 placeholder) ───────────────────────────
        -- Queue in outbox with target_type='kiln'; the Kiln BGW will pick it up
        -- when Phase 6 lands. Until then it accumulates without delivery.
        IF wh.target_type = 'kiln' THEN
            INSERT INTO flint.webhook_outbox
                (webhook_id, payload, headers, endpoint_url, target_type, agui_run_id, status)
            VALUES
                (wh.id, payload, '{}'::jsonb, wh.target_url,
                 'kiln', wh.agui_run_id, 'pending');
            CONTINUE;
        END IF;

        -- ── url target (original behaviour) ─────────────────────────────────
        -- HMAC-SHA256 via pgcrypto. wh.secret is NEVER logged or returned.
        sig_hex := 'sha256=' || encode(
            hmac(payload_txt, wh.secret, 'sha256'),
            'hex'
        );

        -- Assemble standard headers.
        headers := jsonb_build_object(
            'Content-Type',      'application/json',
            'X-Forge-Signature', sig_hex,
            'X-Forge-Event',     TG_OP,
            'X-Forge-Table',     TG_TABLE_SCHEMA || '.' || TG_TABLE_NAME
        );

        -- Forward caller JWT only when the registration explicitly opts in.
        -- The bearer value is passed to the endpoint only; it is NEVER logged.
        IF wh.forward_jwt THEN
            bearer := current_setting('request.headers', true)::json->>'authorization';
            IF bearer IS NOT NULL THEN
                headers := headers || jsonb_build_object('Authorization', bearer);
            END IF;
        END IF;

        -- Merge per-webhook custom headers (webhook-specific values override defaults).
        IF wh.custom_headers <> '{}' THEN
            headers := headers || wh.custom_headers;
        END IF;

        -- Route by delivery tier.
        IF wh.tier = 'standard' THEN
            -- Standard tier: fire-and-forget via pg_net.
            PERFORM net.http_post(
                url     := wh.target_url,
                body    := payload_txt::bytea,
                headers := headers
            );
        ELSE
            -- Durable tier: queue in outbox.
            INSERT INTO flint.webhook_outbox
                (webhook_id, payload, headers, endpoint_url, target_type, status)
            VALUES
                (wh.id, payload, headers, wh.target_url, 'url', 'pending');
        END IF;
    END LOOP;

    -- For DELETE, NEW is NULL; return OLD so the trigger chain stays intact.
    RETURN COALESCE(NEW, OLD);
END;
$$;

-- Example: bind the dispatch trigger to a table (template for operators).
-- CREATE TRIGGER flint_dispatch
--   AFTER INSERT OR UPDATE OR DELETE ON public.some_table
--   FOR EACH ROW EXECUTE FUNCTION flint.dispatch_webhook();

-- Idempotent: add updated_at to webhook_outbox if not present.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE  table_schema = 'flint'
          AND  table_name   = 'webhook_outbox'
          AND  column_name  = 'updated_at'
    ) THEN
        ALTER TABLE flint.webhook_outbox ADD COLUMN updated_at timestamptz;
    END IF;
END
$$;

-- ── Durable webhook dispatcher ───────────────────────────────────────────────
-- Called by pg_cron every minute (wired in 01-extensions.sql after p1-c003).
-- Processes up to 100 pending/retryable outbox entries per invocation using
-- SKIP LOCKED to allow concurrent worker calls without contention.
-- Skips 'kiln' target_type entries — those await Phase 6 Kiln BGW.
--
-- Retry schedule (retry_count → next visible_at delay):
--   0 → 30 s, 1 → 60 s, 2 → 120 s, 3 → 300 s, 4 → 600 s
-- After 5 failures the entry is marked 'failed' and no further retries occur.
CREATE OR REPLACE FUNCTION flint.process_webhook_outbox()
RETURNS int
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = flint, net, public, pg_catalog
AS $$
DECLARE
    entry        record;
    processed    int := 0;
    next_delay   interval;
BEGIN
    FOR entry IN
        SELECT id, webhook_id, payload, headers, endpoint_url,
               target_type, agui_run_id, retry_count
        FROM   flint.webhook_outbox
        WHERE  status      IN ('pending', 'retrying')
          AND  visible_at  <= now()
          AND  target_type != 'kiln'   -- kiln entries wait for Phase 6 BGW
        ORDER  BY visible_at
        LIMIT  100
        FOR UPDATE SKIP LOCKED
    LOOP
        BEGIN
            -- Attempt HTTP delivery via pg_net. The headers jsonb already
            -- contains Content-Type, X-Forge-Signature, and any Authorization.
            PERFORM net.http_post(
                url     := entry.endpoint_url,
                body    := (entry.payload)::text::bytea,
                headers := entry.headers
            );

            -- Mark delivered on success.
            UPDATE flint.webhook_outbox
            SET    status     = 'delivered',
                   updated_at = now()
            WHERE  id = entry.id;

        EXCEPTION WHEN OTHERS THEN
            -- Delivery failed. Apply exponential backoff or mark failed.
            IF entry.retry_count >= 4 THEN
                UPDATE flint.webhook_outbox
                SET    status      = 'failed',
                       retry_count = entry.retry_count + 1,
                       updated_at  = now()
                WHERE  id = entry.id;
            ELSE
                next_delay := CASE entry.retry_count
                    WHEN 0 THEN '30 seconds'::interval
                    WHEN 1 THEN '60 seconds'::interval
                    WHEN 2 THEN '120 seconds'::interval
                    WHEN 3 THEN '300 seconds'::interval
                    ELSE         '600 seconds'::interval
                END;

                UPDATE flint.webhook_outbox
                SET    status      = 'retrying',
                       retry_count = entry.retry_count + 1,
                       visible_at  = now() + next_delay,
                       updated_at  = now()
                WHERE  id = entry.id;
            END IF;
        END;

        processed := processed + 1;
    END LOOP;

    RETURN processed;
END;
$$;
