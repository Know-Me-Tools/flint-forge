-- 0012_a2ui_change_notify.sql
-- p14-c003: A2UI catalog hot-reload — NOTIFY meta_runtime on component/application changes.
--
-- WHY THIS EXISTS
--   The StateManager (crates/fdb-reflection/src/state_manager.rs) listens on the
--   `meta_runtime` Postgres NOTIFY channel and re-compiles the full schema state
--   when it fires. DDL changes (table/column add/drop) already trigger this
--   channel via migration 0007_change_notify.sql. A2UI catalog changes
--   (INSERT/UPDATE/DELETE on flint_a2ui.components / flint_a2ui.applications)
--   did NOT — so adding or editing an A2UI component required a service restart
--   before the gateway served the updated catalog.
--
-- CONTRACT WITH THE LISTENER
--   The payload is a fixed, human-readable string ('a2ui_change'). The listener
--   does not parse it — it simply triggers a full recompile. The recompile path
--   re-reads the catalog under full RLS context, so the trigger payload carries
--   no row image and no tenant identifier (kept tiny on purpose).
--
-- 8000-BYTE NOTIFY LIMIT
--   The payload is a constant 12-byte literal — orders of magnitude under the
--   Postgres 8000-byte NOTIFY cap. No overflow guard is required. (Compare to
--   0007_change_notify.sql, which must guard because it carries full row images.)
--
-- IDEMPOTENCY
--   CREATE SCHEMA IF NOT EXISTS; CREATE OR REPLACE FUNCTION; DROP TRIGGER IF
--   EXISTS + CREATE TRIGGER (Postgres has no portable CREATE TRIGGER IF NOT
--   EXISTS, matching the convention established in 0003_a2ui_triggers.sql).

-- The flint_a2ui schema is created by 0002_flint_a2ui.sql. Guard anyway for
-- ordering safety across re-runs.
CREATE SCHEMA IF NOT EXISTS flint_a2ui;

-- ── Trigger function: notify the StateManager's `meta_runtime` listener ──────
-- Fires AFTER INSERT/UPDATE/DELETE on flint_a2ui.components and
-- flint_a2ui.applications. The payload is a fixed literal so the listener can
-- distinguish the source of the change in logs without parsing structured JSON.
CREATE OR REPLACE FUNCTION flint_a2ui.notify_meta_runtime()
    RETURNS trigger
    LANGUAGE plpgsql
AS $$
BEGIN
    -- Payload identifies the change source for log triage only; the listener
    -- always triggers a full recompile regardless of payload content.
    PERFORM pg_notify('meta_runtime', 'a2ui_change');

    -- AFTER trigger: return value is ignored. RETURN NULL is conventional.
    RETURN NULL;
END;
$$;

COMMENT ON FUNCTION flint_a2ui.notify_meta_runtime() IS
    'AFTER-ROW trigger: pg_notify(''meta_runtime'', ''a2ui_change'') so the fdb-gateway StateManager hot-swaps its compiled A2UI catalog. Wire this on any A2UI catalog table whose changes must reach connected agent frontends without a service restart.';

-- ── Trigger attachment: flint_a2ui.components ────────────────────────────────
-- Components table created by 0002_flint_a2ui.sql. Component add/edit/delete
-- is the dominant hot-reload use case (registry edits in the field).
DROP TRIGGER IF EXISTS a2ui_components_meta_runtime ON flint_a2ui.components;

CREATE TRIGGER a2ui_components_meta_runtime
    AFTER INSERT OR UPDATE OR DELETE ON flint_a2ui.components
    FOR EACH ROW
    EXECUTE FUNCTION flint_a2ui.notify_meta_runtime();

COMMENT ON TRIGGER a2ui_components_meta_runtime ON flint_a2ui.components IS
    'Notifies the StateManager (meta_runtime channel) to recompile the A2UI catalog whenever a component row changes.';

-- ── Trigger attachment: flint_a2ui.applications ──────────────────────────────
-- Applications table created by 0008_flint_a2ui_application_model.sql. Role
-- assignments and application metadata changes affect surface assembly, so
-- they must also propagate to live sessions.
DROP TRIGGER IF EXISTS a2ui_applications_meta_runtime ON flint_a2ui.applications;

CREATE TRIGGER a2ui_applications_meta_runtime
    AFTER INSERT OR UPDATE OR DELETE ON flint_a2ui.applications
    FOR EACH ROW
    EXECUTE FUNCTION flint_a2ui.notify_meta_runtime();

COMMENT ON TRIGGER a2ui_applications_meta_runtime ON flint_a2ui.applications IS
    'Notifies the StateManager (meta_runtime channel) to recompile the A2UI catalog whenever an application row changes.';
