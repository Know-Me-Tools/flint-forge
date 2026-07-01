# p5-c003 — Auto-Binding Trigger

**Phase:** 5 — Flint A2UI Component Registry  
**Priority:** P0  
**Depends on:** p5-c001, p5-c002, Phase 1 (`flint_meta.cache_tables` live)  
**Blocks:** p5-c007 (event assembler uses bindings)

---

## What this change delivers

A PostgreSQL trigger on `flint_meta.cache_tables` that fires on `INSERT` and automatically generates `flint_a2ui.bindings` records — associating database tables with the appropriate base components (form, grid, detail, card). This is the live integration point between `flint_meta` (schema reflection) and `flint_a2ui` (component registry).

### Auto-binding trigger

```sql
CREATE OR REPLACE FUNCTION flint_a2ui.auto_generate_bindings()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE
    v_grid_id   uuid;
    v_form_id   uuid;
    v_detail_id uuid;
BEGIN
    -- Look up base component IDs
    SELECT id INTO v_grid_id   FROM flint_a2ui.components WHERE slug = 'data-grid'   AND is_base = true;
    SELECT id INTO v_form_id   FROM flint_a2ui.components WHERE slug = 'form'        AND is_base = true;
    SELECT id INTO v_detail_id FROM flint_a2ui.components WHERE slug = 'detail-view' AND is_base = true;

    -- Grid binding: all tables get a grid
    INSERT INTO flint_a2ui.bindings (table_schema, table_name, component_id, binding_type, auto_generated, config)
    VALUES (
        NEW.table_schema,
        NEW.table_name,
        v_grid_id,
        'grid',
        true,
        jsonb_build_object(
            'data_source', NEW.table_schema || '.' || NEW.table_name,
            'auto_columns', true
        )
    )
    ON CONFLICT (table_schema, table_name, binding_type) DO UPDATE
        SET config = EXCLUDED.config,
            component_id = EXCLUDED.component_id;

    -- Form binding: tables with writable columns get a form (exclude views/system)
    IF NEW.table_type = 'BASE TABLE' THEN
        INSERT INTO flint_a2ui.bindings (table_schema, table_name, component_id, binding_type, auto_generated, config)
        VALUES (
            NEW.table_schema,
            NEW.table_name,
            v_form_id,
            'form',
            true,
            jsonb_build_object(
                'table', NEW.table_schema || '.' || NEW.table_name,
                'auto_fields', true,
                'submit_action', 'create'
            )
        )
        ON CONFLICT (table_schema, table_name, binding_type) DO UPDATE
            SET config = EXCLUDED.config;
    END IF;

    -- Detail binding: all tables
    INSERT INTO flint_a2ui.bindings (table_schema, table_name, component_id, binding_type, auto_generated, config)
    VALUES (
        NEW.table_schema,
        NEW.table_name,
        v_detail_id,
        'detail',
        true,
        jsonb_build_object(
            'table', NEW.table_schema || '.' || NEW.table_name,
            'auto_fields', true
        )
    )
    ON CONFLICT (table_schema, table_name, binding_type) DO UPDATE
        SET config = EXCLUDED.config;

    -- Log the auto-generation event
    INSERT INTO flint_a2ui.events (event_type, actor, object, action, result, details)
    VALUES (
        'binding_auto_generated',
        'system',
        NEW.table_schema || '.' || NEW.table_name,
        'create',
        true,
        jsonb_build_object(
            'table_schema', NEW.table_schema,
            'table_name', NEW.table_name,
            'binding_types', ARRAY['grid', 'form', 'detail']
        )
    );

    RETURN NEW;
END;
$$;

CREATE TRIGGER a2ui_auto_bind_tables
    AFTER INSERT ON flint_meta.cache_tables
    FOR EACH ROW
    EXECUTE FUNCTION flint_a2ui.auto_generate_bindings();
```

### Column type → input component mapping function

```sql
CREATE OR REPLACE FUNCTION flint_a2ui.column_type_to_component(pg_type text)
RETURNS text LANGUAGE sql IMMUTABLE AS $$
    SELECT CASE
        WHEN pg_type IN ('text', 'varchar', 'char', 'name')          THEN 'text-input'
        WHEN pg_type IN ('int2', 'int4', 'int8', 'float4', 'float8') THEN 'number-input'
        WHEN pg_type = 'bool'                                          THEN 'toggle'
        WHEN pg_type IN ('date', 'timestamp', 'timestamptz')          THEN 'date-picker'
        WHEN pg_type = 'jsonb'                                         THEN 'json-viewer'
        WHEN pg_type = 'uuid'                                          THEN 'text-input'
        ELSE 'text-input'
    END
$$;
```

### `flint_meta.agui_descriptor()` correction (part of this change)

The existing `flint_meta.agui_descriptor()` function (p1-c010) returns `'protocol': 'ag-ui/1.0'`. This label is technically wrong — AG-UI is the transport layer, not the content protocol. In this change, the function is patched to:

```rust
// In ext-flint-meta/src/agui.rs — update the protocol field
// OLD: "protocol": "ag-ui/1.0"
// NEW: "protocol": "flint-forge/schema-descriptor/1.0"
// The descriptor is now a Flint-specific format, not an AG-UI claim
```

This is a backward-compatible label change. No callers depend on the exact string value (the function was only used in the agui_descriptor tests and as a CompiledState seed).

---

## Gate test

- Insert a test row into `flint_meta.cache_tables`; verify within 5 seconds:
  - `flint_a2ui.bindings` contains rows with `table_name = <test_table>` and `binding_type IN ('grid', 'form', 'detail')`
  - `flint_a2ui.events` contains a `binding_auto_generated` row for the test table
- Verify `flint_a2ui.column_type_to_component('text')` = `'text-input'`
- Verify `flint_a2ui.column_type_to_component('bool')` = `'toggle'`
- Verify `flint_meta.agui_descriptor()` returns `'protocol': 'flint-forge/schema-descriptor/1.0'`
