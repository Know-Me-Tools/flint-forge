-- 0007_change_notify.sql
-- Flint Quarry — LISTEN/NOTIFY change feed (OQ-FRF-1 workaround).
--
-- WHY THIS EXISTS
--   The upstream Flint Realtime Fabric `WatchEntityType` gRPC RPC is not yet
--   shipped. Until it is, real-time GraphQL subscriptions need a source of raw
--   change events. This migration installs an in-process Postgres LISTEN/NOTIFY
--   feed that the `fdb-realtime::ListenChangeSource` adapter consumes.
--
-- CONTRACT WITH THE ADAPTER / USE-CASE (critical — do not weaken)
--   The delivered row is NEVER trusted from this payload. `fdb-app`'s
--   `Quarry::subscribe_rls_filtered` re-queries every changed row under full
--   RLS context (`build_pk_filters` -> RestExecutor round-trip) before delivery.
--   Therefore the NOTIFY payload only needs to carry enough to:
--     (a) identify the table + operation, and
--     (b) reconstruct the PRIMARY KEY so the row can be re-fetched.
--   The full `record`/`old_record` images below are an optimization the RLS
--   re-query overwrites; they are never the authoritative delivered row.
--
-- 8000-BYTE NOTIFY LIMIT (the load-bearing safety choice)
--   Postgres caps NOTIFY payloads at 8000 bytes. If pg_notify() is handed a
--   larger payload it RAISES inside the triggering transaction — which would
--   abort the user's INSERT/UPDATE. A subscription workaround must NEVER break
--   writes. So this function builds the full payload, measures it, and if it
--   exceeds a safe threshold (7500 bytes, leaving headroom under 8000 for
--   channel framing) it degrades to a PRIMARY-KEY-ONLY image and sets
--   truncated=true. The PK-only image is guaranteed small and still satisfies
--   the re-query contract above. A table with no PK degrades to record=null /
--   old_record=null; the downstream re-query simply finds nothing to re-fetch,
--   which is a missed notification (never a leaked row).
--
-- CHANNEL
--   All opted-in tables NOTIFY on ONE fixed channel `flint_change`. Postgres
--   channel names are IDENTIFIER-limited (63 bytes) and cannot safely encode an
--   arbitrary <schema>.<table>+tenant key, so selectivity lives in the payload
--   and the Rust fan-out filters by entity_type ("<schema>.<table>"). One
--   channel => one LISTEN => one broadcast, which is what makes fan-out tractable.
--
-- IDEMPOTENCY
--   CREATE SCHEMA IF NOT EXISTS; CREATE OR REPLACE FUNCTION; procedure uses
--   DROP TRIGGER IF EXISTS + CREATE TRIGGER (Postgres has no portable
--   CREATE TRIGGER IF NOT EXISTS, so drop-then-create is the convention here,
--   matching 0003_a2ui_triggers.sql).

CREATE SCHEMA IF NOT EXISTS flint;

-- ── Helper: project a row image down to only its primary-key columns ─────────
-- Used to build the small PK-only fallback image when the full payload would
-- overflow the NOTIFY limit. IMMUTABLE + STRICT-style guards: returns NULL when
-- either the image or the column list is NULL/empty (e.g. DELETE has no NEW
-- image; a PK-less table yields an empty cols array). Only the named keys are
-- copied out of the source jsonb, so the result is bounded by the PK width.
CREATE OR REPLACE FUNCTION flint.pk_only(img jsonb, cols text[])
    RETURNS jsonb
    LANGUAGE sql
    IMMUTABLE
AS $$
    SELECT CASE
        WHEN img IS NULL OR cols IS NULL OR cardinality(cols) = 0 THEN NULL
        ELSE (
            -- Rebuild an object containing only the PK keys present in `img`.
            -- jsonb_object_agg over the intersection of cols and the image keys.
            SELECT jsonb_object_agg(k, img -> k)
            FROM unnest(cols) AS k
            WHERE img ? k
        )
    END
$$;

COMMENT ON FUNCTION flint.pk_only(jsonb, text[]) IS
    'Projects a row image (jsonb) down to only the named primary-key columns; NULL when image or cols is NULL/empty. Used for the truncated NOTIFY fallback.';

-- ── Trigger function: emit a self-describing JSON change event ───────────────
CREATE OR REPLACE FUNCTION flint.notify_change()
    RETURNS trigger
    LANGUAGE plpgsql
AS $$
DECLARE
    -- lower(TG_OP) yields insert | update | delete. Maps to ChangeOp in Rust.
    -- NOTE: there is no native UPSERT trigger op — an INSERT ... ON CONFLICT DO
    -- UPDATE fires as INSERT or UPDATE, so this adapter never emits "upsert".
    -- The Upsert variant remains in the domain enum for FRF parity only.
    v_op       text  := lower(TG_OP);
    -- NEW is absent on DELETE; OLD is absent on INSERT. Guard both.
    v_new      jsonb := CASE WHEN TG_OP <> 'DELETE' THEN to_jsonb(NEW) END;
    v_old      jsonb := CASE WHEN TG_OP <> 'INSERT' THEN to_jsonb(OLD) END;
    -- Best-effort tenant extraction from the row's tenant_id column, if any.
    -- NULL for tables that are not tenant-scoped — the fan-out then does not
    -- tenant-pre-filter, and RLS remains the authoritative tenant gate.
    v_tenant   text  := COALESCE(v_new ->> 'tenant_id', v_old ->> 'tenant_id');
    v_payload  jsonb;
    v_pk_cols  text[];
BEGIN
    -- Build the full payload first.
    v_payload := jsonb_build_object(
        'op',         v_op,
        'schema',     TG_TABLE_SCHEMA,
        'table',      TG_TABLE_NAME,
        'tenant',     v_tenant,
        'record',     v_new,
        'old_record', v_old,
        'truncated',  false
    );

    -- Overflow guard. octet_length on the JSON text is the byte size the
    -- NOTIFY payload will actually occupy; 7500 leaves headroom under the
    -- hard 8000-byte limit for channel framing.
    IF octet_length(v_payload::text) > 7500 THEN
        -- Resolve this table's PRIMARY KEY columns in key order. We read the
        -- catalog directly (rather than information_schema) so the lookup is
        -- fast and works for any schema/table the trigger is attached to.
        -- unnest(i.indkey) WITH ORDINALITY preserves the composite-key order;
        -- we join back to pg_attribute to turn attnums into column names.
        SELECT array_agg(a.attname ORDER BY k.ord)
          INTO v_pk_cols
          FROM pg_index i
          JOIN pg_class c        ON c.oid = i.indrelid
          JOIN pg_namespace n    ON n.oid = c.relnamespace
          CROSS JOIN LATERAL unnest(i.indkey) WITH ORDINALITY AS k(attnum, ord)
          JOIN pg_attribute a    ON a.attrelid = c.oid AND a.attnum = k.attnum
         WHERE i.indisprimary
           AND n.nspname = TG_TABLE_SCHEMA
           AND c.relname = TG_TABLE_NAME;

        -- Rebuild with PK-only images. flint.pk_only returns NULL when there is
        -- no PK (v_pk_cols is NULL) — a PK-less wide row therefore notifies with
        -- record/old_record = null and truncated = true; downstream simply has
        -- nothing to re-fetch (a missed notification, never a leaked row).
        v_payload := jsonb_build_object(
            'op',         v_op,
            'schema',     TG_TABLE_SCHEMA,
            'table',      TG_TABLE_NAME,
            'tenant',     v_tenant,
            'record',     flint.pk_only(v_new, v_pk_cols),
            'old_record', flint.pk_only(v_old, v_pk_cols),
            'truncated',  true
        );
    END IF;

    PERFORM pg_notify('flint_change', v_payload::text);

    -- AFTER trigger: the return value is ignored. RETURN NULL is conventional.
    RETURN NULL;
END;
$$;

COMMENT ON FUNCTION flint.notify_change() IS
    'AFTER-ROW trigger: pg_notify(''flint_change'', json) for each change. Degrades to a primary-key-only payload when the full image would exceed 7500 bytes so it can never abort the triggering write. The downstream RLS re-query re-fetches the full row, so the payload is only used to identify the table/op and reconstruct the PK filter.';

-- ── Convenience: attach the notify trigger to a table (idempotent) ───────────
-- Operators enable the feed per table explicitly — we deliberately do NOT
-- auto-attach to every table (that would notify on system/internal tables).
-- Identifiers are quoted with format(%I) to guard against injection and to
-- handle mixed-case / reserved names safely. DROP-then-CREATE makes re-running
-- safe (no portable CREATE TRIGGER IF NOT EXISTS).
CREATE OR REPLACE PROCEDURE flint.enable_change_notify(p_schema text, p_table text)
    LANGUAGE plpgsql
AS $$
BEGIN
    EXECUTE format(
        'DROP TRIGGER IF EXISTS flint_notify_change ON %I.%I',
        p_schema, p_table
    );
    EXECUTE format(
        'CREATE TRIGGER flint_notify_change '
        'AFTER INSERT OR UPDATE OR DELETE ON %I.%I '
        'FOR EACH ROW EXECUTE FUNCTION flint.notify_change()',
        p_schema, p_table
    );
END;
$$;

COMMENT ON PROCEDURE flint.enable_change_notify(text, text) IS
    'Attaches (idempotently) the flint_notify_change AFTER-ROW trigger to the given table so its changes are published on the flint_change NOTIFY channel. Call once per table an operator wants to expose to real-time subscriptions.';

-- ── Opt-in template ──────────────────────────────────────────────────────────
-- Enable a table two equivalent ways. Prefer the procedure:
--
--   CALL flint.enable_change_notify('public', 'documents');
--
-- Or attach the trigger directly (kept as a commented template so this
-- migration ships the function + helper only, never an auto-attachment):
--
--   DROP TRIGGER IF EXISTS flint_notify_change ON public.documents;
--   CREATE TRIGGER flint_notify_change
--     AFTER INSERT OR UPDATE OR DELETE ON public.documents
--     FOR EACH ROW EXECUTE FUNCTION flint.notify_change();
