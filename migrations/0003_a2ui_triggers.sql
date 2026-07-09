-- Migration 0003: A2UI auto-binding trigger and column-type mapping
-- Depends on: 0001_flint_meta (flint_meta.cache_tables), 0002_flint_a2ui (flint_a2ui schema)
-- Idempotent: CREATE OR REPLACE FUNCTION + CREATE TRIGGER uses IF NOT EXISTS guard

-- ── column_type_to_component ────────────────────────────────────────────────
-- Maps a Postgres type name to the most appropriate Flint A2UI input component.
-- IMMUTABLE: same input always produces same output (no DB side-effects).

CREATE OR REPLACE FUNCTION flint_a2ui.column_type_to_component(pg_type text)
RETURNS text LANGUAGE sql IMMUTABLE AS $$
    SELECT CASE
        WHEN pg_type IN ('text', 'varchar', 'bpchar', 'char', 'name', 'citext')    THEN 'text-input'
        WHEN pg_type IN ('int2', 'int4', 'int8', 'float4', 'float8', 'numeric')    THEN 'number-input'
        WHEN pg_type = 'bool'                                                        THEN 'toggle'
        WHEN pg_type IN ('date', 'timestamp', 'timestamptz', 'timetz', 'time')      THEN 'date-picker'
        WHEN pg_type IN ('jsonb', 'json')                                            THEN 'json-viewer'
        WHEN pg_type = 'uuid'                                                        THEN 'text-input'
        ELSE 'text-input'
    END
$$;

COMMENT ON FUNCTION flint_a2ui.column_type_to_component(text) IS
    'Maps a Postgres type name to the most appropriate Flint A2UI input component slug.';

-- ── auto_generate_bindings ──────────────────────────────────────────────────
-- Trigger function fired AFTER INSERT on flint_meta.cache_tables.
-- Generates grid, form (BASE TABLE only), and detail bindings for the new table.
-- All inserts are idempotent via ON CONFLICT (table_schema, table_name, binding_type).

CREATE OR REPLACE FUNCTION flint_a2ui.auto_generate_bindings()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE
    v_grid_id   uuid;
    v_form_id   uuid;
    v_detail_id uuid;
    v_table_type text;
BEGIN
    -- Look up base component IDs (resolved once per trigger invocation)
    SELECT id INTO v_grid_id   FROM flint_a2ui.components WHERE slug = 'data-grid'   AND is_base = true;
    SELECT id INTO v_form_id   FROM flint_a2ui.components WHERE slug = 'form'        AND is_base = true;
    SELECT id INTO v_detail_id FROM flint_a2ui.components WHERE slug = 'detail-view' AND is_base = true;

    -- Skip binding generation if base components have not been seeded yet
    IF v_grid_id IS NULL OR v_form_id IS NULL OR v_detail_id IS NULL THEN
        RETURN NEW;
    END IF;

    -- cache_tables uses schema_name/table_name/is_view; bindings use table_schema/table_name.
    v_table_type := CASE WHEN NEW.is_view THEN 'VIEW' ELSE 'BASE TABLE' END;

    -- Grid binding: every table/view gets a data-grid
    INSERT INTO flint_a2ui.bindings
        (table_schema, table_name, component_id, binding_type, auto_generated, config)
    VALUES (
        NEW.schema_name,
        NEW.table_name,
        v_grid_id,
        'grid',
        true,
        jsonb_build_object(
            'data_source', NEW.schema_name || '.' || NEW.table_name,
            'auto_columns', true
        )
    )
    ON CONFLICT (table_schema, table_name, binding_type) DO UPDATE
        SET config       = EXCLUDED.config,
            component_id = EXCLUDED.component_id;

    -- Form binding: writable base tables only (not views or system tables)
    IF NOT NEW.is_view THEN
        INSERT INTO flint_a2ui.bindings
            (table_schema, table_name, component_id, binding_type, auto_generated, config)
        VALUES (
            NEW.schema_name,
            NEW.table_name,
            v_form_id,
            'form',
            true,
            jsonb_build_object(
                'table',         NEW.schema_name || '.' || NEW.table_name,
                'auto_fields',   true,
                'submit_action', 'create'
            )
        )
        ON CONFLICT (table_schema, table_name, binding_type) DO UPDATE
            SET config = EXCLUDED.config;
    END IF;

    -- Detail binding: every table/view
    INSERT INTO flint_a2ui.bindings
        (table_schema, table_name, component_id, binding_type, auto_generated, config)
    VALUES (
        NEW.schema_name,
        NEW.table_name,
        v_detail_id,
        'detail',
        true,
        jsonb_build_object(
            'table',       NEW.schema_name || '.' || NEW.table_name,
            'auto_fields', true
        )
    )
    ON CONFLICT (table_schema, table_name, binding_type) DO UPDATE
        SET config = EXCLUDED.config;

    -- Audit event (append-only; events table has no UPDATE/DELETE policies)
    INSERT INTO flint_a2ui.events
        (event_type, actor, object, action, result, details)
    VALUES (
        'binding_auto_generated',
        'system',
        NEW.schema_name || '.' || NEW.table_name,
        'create',
        true,
        jsonb_build_object(
            'table_schema',  NEW.schema_name,
            'table_name',    NEW.table_name,
            'table_type',    v_table_type,
            'binding_types', CASE WHEN NOT NEW.is_view
                                  THEN ARRAY['grid', 'form', 'detail']
                                  ELSE ARRAY['grid', 'detail']
                             END
        )
    );

    RETURN NEW;
END;
$$;

COMMENT ON FUNCTION flint_a2ui.auto_generate_bindings() IS
    'Trigger: generates grid/form/detail bindings when a new table is cached in flint_meta.cache_tables.';

-- ── Trigger (DROP + CREATE to ensure idempotency across migrations) ──────────
DROP TRIGGER IF EXISTS a2ui_auto_bind_tables ON flint_meta.cache_tables;

CREATE TRIGGER a2ui_auto_bind_tables
    AFTER INSERT ON flint_meta.cache_tables
    FOR EACH ROW
    EXECUTE FUNCTION flint_a2ui.auto_generate_bindings();

COMMENT ON TRIGGER a2ui_auto_bind_tables ON flint_meta.cache_tables IS
    'Fires after each INSERT into flint_meta.cache_tables to auto-generate A2UI component bindings.';

-- ── a2ui_embed_notify ───────────────────────────────────────────────────────
-- Trigger function: notify the fdb-gateway A2UI embedder when a new component
-- is inserted. The payload is the component id; the Rust listener calls
-- llm.embed() and writes the vector into flint_a2ui.embeddings.

CREATE OR REPLACE FUNCTION flint_a2ui.embed_notify()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    PERFORM pg_notify('a2ui_embed', NEW.id::text);
    RETURN NEW;
END;
$$;

COMMENT ON FUNCTION flint_a2ui.embed_notify() IS
    'Trigger: emits pg_notify on a2ui_embed channel when a component is inserted.';

DROP TRIGGER IF EXISTS a2ui_component_embed_notify ON flint_a2ui.components;

CREATE TRIGGER a2ui_component_embed_notify
    AFTER INSERT ON flint_a2ui.components
    FOR EACH ROW
    EXECUTE FUNCTION flint_a2ui.embed_notify();

COMMENT ON TRIGGER a2ui_component_embed_notify ON flint_a2ui.components IS
    'Notifies the A2UI embedder background task to generate a vector embedding.';
