//! DDL event trigger declarations for ext-flint-meta.
//!
//! The trigger logic runs in PL/pgSQL to avoid pgrx event trigger FFI complexity.
//! This module contains only `extension_sql!` declarations; no Rust functions are
//! exported. The SQL is ordered after the bootstrap tables via the `requires`
//! attribute.
//!
//! ## Trigger summary
//!
//! | Trigger | Event | Cache target |
//! |---------|-------|--------------|
//! | `flint_meta_ddl_refresh`    | `ddl_command_end` | cache_tables, cache_functions, cache_types |
//! | `flint_meta_ddl_invalidate` | `sql_drop`        | cache_tables, cache_functions, cache_types |
//!
//! Both triggers write a row to `flint_meta.schema_version` and emit a
//! `pg_notify('meta_runtime', …)` so the reflection engine can invalidate its
//! in-process cache without polling.
//!
//! `flint_meta.full_refresh()` is the reconciliation escape-hatch: it truncates
//! all `cache_*` tables and repopulates them from `pg_catalog`. Call it after any
//! DDL that the incremental triggers do not cover (see `meta-trigger-coverage.md`).

use pgrx::prelude::*;

extension_sql!(
    r#"
-- ── refresh_cache(): fired on ddl_command_end ─────────────────────────────
CREATE OR REPLACE FUNCTION flint_meta.refresh_cache()
RETURNS event_trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = flint_meta, pg_catalog
AS $$
DECLARE
    obj          record;
    v_ver        bigint;
    skip_schemas text[] := ARRAY['flint_meta', 'vault', 'pg_catalog',
                                  'information_schema', 'pg_toast'];
BEGIN
    FOR obj IN SELECT * FROM pg_event_trigger_ddl_commands() LOOP
        -- Skip internal schema DDL to avoid self-invalidation loops.
        CONTINUE WHEN obj.schema_name = ANY(skip_schemas);
        CONTINUE WHEN obj.object_identity ILIKE '%ext-flint-meta%';

        IF obj.command_tag IN ('CREATE TABLE', 'ALTER TABLE') THEN
            INSERT INTO flint_meta.cache_tables
                        (schema_name, table_name, is_view, rls_enabled, updated_at)
            SELECT n.nspname,
                   c.relname,
                   c.relkind = 'v',
                   c.relrowsecurity,
                   now()
            FROM   pg_class     c
            JOIN   pg_namespace n ON n.oid = c.relnamespace
            WHERE  n.nspname = obj.schema_name
              AND  c.relname = split_part(obj.object_identity, '.', 2)
            ON CONFLICT (schema_name, table_name) DO UPDATE
              SET rls_enabled = EXCLUDED.rls_enabled,
                  updated_at  = now();

        ELSIF obj.command_tag IN ('CREATE VIEW', 'ALTER VIEW') THEN
            INSERT INTO flint_meta.cache_tables
                        (schema_name, table_name, is_view, rls_enabled, updated_at)
            SELECT n.nspname, c.relname, true, false, now()
            FROM   pg_class     c
            JOIN   pg_namespace n ON n.oid = c.relnamespace
            WHERE  n.nspname = obj.schema_name
              AND  c.relname = split_part(obj.object_identity, '.', 2)
            ON CONFLICT (schema_name, table_name) DO UPDATE
              SET is_view    = true,
                  updated_at = now();

        ELSIF obj.command_tag IN ('CREATE FUNCTION',
                                   'CREATE OR REPLACE FUNCTION') THEN
            INSERT INTO flint_meta.cache_functions
                        (schema_name, function_name, return_type,
                         argument_types, is_stable)
            SELECT n.nspname,
                   p.proname,
                   pg_catalog.format_type(p.prorettype, null),
                   ARRAY(SELECT pg_catalog.format_type(unnest(p.proargtypes), null)),
                   p.provolatile = 's'
            FROM   pg_proc      p
            JOIN   pg_namespace n ON n.oid = p.pronamespace
            WHERE  n.nspname = obj.schema_name
              AND  p.proname = split_part(obj.object_identity, '.', 2)
            ON CONFLICT (schema_name, function_name, argument_types) DO UPDATE
              SET return_type = EXCLUDED.return_type,
                  is_stable   = EXCLUDED.is_stable;

        ELSIF obj.command_tag = 'CREATE TYPE' THEN
            INSERT INTO flint_meta.cache_types (schema_name, type_name, kind)
            SELECT n.nspname,
                   t.typname,
                   CASE t.typtype
                       WHEN 'e' THEN 'enum'
                       WHEN 'c' THEN 'composite'
                       WHEN 'd' THEN 'domain'
                       ELSE          'base'
                   END
            FROM   pg_type      t
            JOIN   pg_namespace n ON n.oid = t.typnamespace
            WHERE  n.nspname = obj.schema_name
              AND  t.typname = split_part(obj.object_identity, '.', 2)
            ON CONFLICT (schema_name, type_name) DO NOTHING;
        END IF;
    END LOOP;

    -- Record the DDL event and increment the schema version.
    INSERT INTO flint_meta.schema_version (ddl_tag, object_name)
    VALUES (
        (SELECT string_agg(DISTINCT command_tag, ',')
         FROM   pg_event_trigger_ddl_commands()),
        (SELECT string_agg(DISTINCT object_identity, ',')
         FROM   pg_event_trigger_ddl_commands()
         WHERE  schema_name <> ALL(skip_schemas))
    )
    RETURNING version INTO v_ver;

    -- Notify the reflection engine so it can invalidate its in-process cache.
    PERFORM pg_notify(
        'meta_runtime',
        json_build_object(
            'version',     v_ver,
            'ddl_tag',     (SELECT string_agg(DISTINCT command_tag, ',')
                            FROM pg_event_trigger_ddl_commands()),
            'object_name', (SELECT string_agg(DISTINCT object_identity, ',')
                            FROM pg_event_trigger_ddl_commands()
                            WHERE schema_name <> ALL(skip_schemas))
        )::text
    );
END;
$$;

-- ── invalidate_cache(): fired on sql_drop ──────────────────────────────────
CREATE OR REPLACE FUNCTION flint_meta.invalidate_cache()
RETURNS event_trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = flint_meta, pg_catalog
AS $$
DECLARE
    obj          record;
    v_ver        bigint;
    skip_schemas text[] := ARRAY['flint_meta', 'vault', 'pg_catalog',
                                  'information_schema', 'pg_toast'];
BEGIN
    FOR obj IN SELECT * FROM pg_event_trigger_dropped_objects() LOOP
        CONTINUE WHEN obj.schema_name = ANY(skip_schemas);

        IF obj.object_type = 'table' THEN
            DELETE FROM flint_meta.cache_tables
            WHERE  schema_name = obj.schema_name
              AND  table_name  = obj.object_name;
        ELSIF obj.object_type = 'function' THEN
            DELETE FROM flint_meta.cache_functions
            WHERE  schema_name   = obj.schema_name
              AND  function_name = obj.object_name;
        ELSIF obj.object_type = 'type' THEN
            DELETE FROM flint_meta.cache_types
            WHERE  schema_name = obj.schema_name
              AND  type_name   = obj.object_name;
        END IF;
    END LOOP;

    INSERT INTO flint_meta.schema_version (ddl_tag, object_name)
    VALUES (
        'DROP',
        (SELECT string_agg(object_name, ',')
         FROM   pg_event_trigger_dropped_objects()
         WHERE  schema_name <> ALL(skip_schemas))
    )
    RETURNING version INTO v_ver;

    PERFORM pg_notify(
        'meta_runtime',
        json_build_object(
            'version',     v_ver,
            'ddl_tag',     'DROP',
            'object_name', (SELECT string_agg(object_name, ',')
                            FROM pg_event_trigger_dropped_objects()
                            WHERE schema_name <> ALL(skip_schemas))
        )::text
    );
END;
$$;

-- ── full_refresh(): truncate + repopulate all cache_* from pg_catalog ──────
CREATE OR REPLACE FUNCTION flint_meta.full_refresh()
RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = flint_meta, pg_catalog
AS $$
DECLARE
    v_ver        bigint;
    skip_schemas text[] := ARRAY['flint_meta', 'vault', 'pg_catalog',
                                  'information_schema', 'pg_toast', 'pg_temp'];
BEGIN
    -- Truncate in dependency order; CASCADE removes child rows via FK.
    TRUNCATE flint_meta.cache_relationships,
             flint_meta.cache_columns,
             flint_meta.cache_policies,
             flint_meta.cache_functions,
             flint_meta.cache_types,
             flint_meta.cache_tables;

    -- Repopulate cache_tables (tables, views, and materialised views).
    INSERT INTO flint_meta.cache_tables
                (schema_name, table_name, is_view, rls_enabled, updated_at)
    SELECT n.nspname, c.relname, c.relkind = 'v', c.relrowsecurity, now()
    FROM   pg_class     c
    JOIN   pg_namespace n ON n.oid = c.relnamespace
    WHERE  c.relkind IN ('r', 'v', 'm')
      AND  n.nspname <> ALL(skip_schemas)
      AND  NOT n.nspname LIKE 'pg_%';

    -- Repopulate cache_columns.
    INSERT INTO flint_meta.cache_columns
                (schema_name, table_name, column_name, data_type,
                 is_nullable, is_pk, is_fk, ordinal)
    SELECT n.nspname,
           c.relname,
           a.attname,
           pg_catalog.format_type(a.atttypid, a.atttypmod),
           NOT a.attnotnull,
           EXISTS (SELECT 1 FROM pg_constraint co
                   WHERE  co.conrelid = c.oid AND co.contype = 'p'
                     AND  a.attnum = ANY(co.conkey)),
           EXISTS (SELECT 1 FROM pg_constraint co
                   WHERE  co.conrelid = c.oid AND co.contype = 'f'
                     AND  a.attnum = ANY(co.conkey)),
           a.attnum
    FROM   pg_attribute a
    JOIN   pg_class     c ON c.oid = a.attrelid
    JOIN   pg_namespace n ON n.oid = c.relnamespace
    WHERE  a.attnum > 0
      AND  NOT a.attisdropped
      AND  c.relkind IN ('r', 'v', 'm')
      AND  n.nspname <> ALL(skip_schemas)
      AND  NOT n.nspname LIKE 'pg_%';

    -- Repopulate cache_relationships (FK constraints, single-column only).
    INSERT INTO flint_meta.cache_relationships
                (from_schema, from_table, from_column,
                 to_schema,   to_table,   to_column, constraint_name)
    SELECT fn.nspname, fc.relname, fa.attname,
           tn.nspname, tc.relname, ta.attname,
           co.conname
    FROM   pg_constraint co
    JOIN   pg_class      fc ON fc.oid = co.conrelid
    JOIN   pg_namespace  fn ON fn.oid = fc.relnamespace
    JOIN   pg_class      tc ON tc.oid = co.confrelid
    JOIN   pg_namespace  tn ON tn.oid = tc.relnamespace
    JOIN   pg_attribute  fa ON fa.attrelid = fc.oid
                            AND fa.attnum  = co.conkey[1]
    JOIN   pg_attribute  ta ON ta.attrelid = tc.oid
                            AND ta.attnum  = co.confkey[1]
    WHERE  co.contype = 'f'
      AND  fn.nspname <> ALL(skip_schemas)
    ON CONFLICT DO NOTHING;

    -- Repopulate cache_functions.
    INSERT INTO flint_meta.cache_functions
                (schema_name, function_name, return_type,
                 argument_types, is_stable)
    SELECT n.nspname,
           p.proname,
           pg_catalog.format_type(p.prorettype, null),
           ARRAY(SELECT pg_catalog.format_type(unnest(p.proargtypes), null)),
           p.provolatile = 's'
    FROM   pg_proc      p
    JOIN   pg_namespace n ON n.oid = p.pronamespace
    WHERE  n.nspname <> ALL(skip_schemas)
      AND  NOT n.nspname LIKE 'pg_%';

    -- Repopulate cache_policies (RLS policies).
    INSERT INTO flint_meta.cache_policies
                (schema_name, table_name, policy_name, command,
                 roles, permissive)
    SELECT n.nspname,
           c.relname,
           pol.polname,
           CASE pol.polcmd
               WHEN 'r' THEN 'SELECT'
               WHEN 'a' THEN 'INSERT'
               WHEN 'w' THEN 'UPDATE'
               WHEN 'd' THEN 'DELETE'
               ELSE          'ALL'
           END,
           ARRAY(SELECT rolname FROM pg_roles WHERE oid = ANY(pol.polroles)),
           pol.polpermissive
    FROM   pg_policy    pol
    JOIN   pg_class     c ON c.oid = pol.polrelid
    JOIN   pg_namespace n ON n.oid = c.relnamespace
    WHERE  n.nspname <> ALL(skip_schemas);

    -- Repopulate cache_types (enums, composites, domains).
    INSERT INTO flint_meta.cache_types (schema_name, type_name, kind, labels)
    SELECT n.nspname,
           t.typname,
           CASE t.typtype
               WHEN 'e' THEN 'enum'
               WHEN 'c' THEN 'composite'
               WHEN 'd' THEN 'domain'
               ELSE          'base'
           END,
           COALESCE(
               ARRAY(SELECT enumlabel FROM pg_enum
                     WHERE  enumtypid = t.oid
                     ORDER  BY enumsortorder),
               '{}'
           )
    FROM   pg_type      t
    JOIN   pg_namespace n ON n.oid = t.typnamespace
    WHERE  t.typtype IN ('e', 'c', 'd')
      AND  n.nspname <> ALL(skip_schemas)
    ON CONFLICT (schema_name, type_name) DO NOTHING;

    -- Record the full refresh and emit a notify.
    INSERT INTO flint_meta.schema_version (ddl_tag, object_name)
    VALUES ('FULL_REFRESH', 'all')
    RETURNING version INTO v_ver;

    PERFORM pg_notify(
        'meta_runtime',
        json_build_object(
            'version',     v_ver,
            'ddl_tag',     'FULL_REFRESH',
            'object_name', 'all'
        )::text
    );
END;
$$;

-- ── Event trigger registrations ────────────────────────────────────────────
CREATE EVENT TRIGGER flint_meta_ddl_refresh
    ON ddl_command_end
    WHEN TAG IN (
        'CREATE TABLE', 'ALTER TABLE',
        'CREATE VIEW',  'ALTER VIEW',
        'CREATE FUNCTION',
        'CREATE TYPE'
    )
    EXECUTE FUNCTION flint_meta.refresh_cache();

CREATE EVENT TRIGGER flint_meta_ddl_invalidate
    ON sql_drop
    EXECUTE FUNCTION flint_meta.invalidate_cache();
"#,
    name = "flint_meta_triggers",
    requires = ["flint_meta_bootstrap"]
);

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_full_refresh_runs() {
        // full_refresh() must complete without error and increment schema_version.
        let v_before: i64 =
            Spi::get_one::<i64>("SELECT COALESCE(MAX(version), 0) FROM flint_meta.schema_version")
                .unwrap_or(None)
                .unwrap_or(0);

        Spi::run("SELECT flint_meta.full_refresh()").unwrap();

        let v_after: i64 =
            Spi::get_one::<i64>("SELECT COALESCE(MAX(version), 0) FROM flint_meta.schema_version")
                .unwrap_or(None)
                .unwrap_or(0);

        assert!(
            v_after > v_before,
            "full_refresh() must increment schema_version"
        );
    }

    #[pg_test]
    fn test_cache_tables_populated_after_refresh() {
        // Create a table, run full_refresh, verify the table appears in cache_tables.
        Spi::run(
            "CREATE TABLE IF NOT EXISTS public.meta_trigger_test_tbl \
             (id int PRIMARY KEY)",
        )
        .unwrap();

        Spi::run("SELECT flint_meta.full_refresh()").unwrap();

        let found: bool = Spi::get_one::<bool>(
            "SELECT EXISTS( \
                SELECT 1 FROM flint_meta.cache_tables \
                WHERE  schema_name = 'public' \
                  AND  table_name  = 'meta_trigger_test_tbl' \
             )",
        )
        .unwrap_or(None)
        .unwrap_or(false);

        assert!(
            found,
            "meta_trigger_test_tbl should appear in cache_tables after full_refresh"
        );

        Spi::run("DROP TABLE IF EXISTS public.meta_trigger_test_tbl").unwrap();
    }
}
