/* ext-flint-meta 0.1.0 → 0.1.1
 *
 * Adds the three RLS-posture columns to flint_meta.cache_tables, widens
 * flint_meta.tables() to return them, and extends the DDL event trigger to fire
 * on policy and grant DDL.
 *
 * WHY THIS FILE EXISTS
 *
 * The 0.1.0 schema was changed in place by an earlier commit: the columns were
 * added to sql/flint_meta.sql and tables() was widened, but default_version
 * stayed at 0.1.0. That is invisible to a fresh `CREATE EXTENSION` (which runs
 * the bootstrap file and gets the new shape) and fatal to an existing install
 * (which runs nothing, because `ALTER EXTENSION ... UPDATE` only ever executes
 * <name>--<from>--<to>.sql scripts — never the bootstrap file, ALTERs inside it
 * included). Existing databases were therefore stranded on the old shape with
 * no upgrade path at all. This script is that path.
 *
 * Written to be safe to run against a database that already has some of these
 * objects, because a 0.1.0 install that was created *after* the in-place schema
 * edit already has the new columns and the new tables() while still reporting
 * version 0.1.0. Both shapes must converge here.
 *
 * ORDERING NOTE
 *
 * The gateway tolerates both tables() signatures (it selects the new columns
 * through a to_jsonb fallback), so extension upgrade and gateway deploy do not
 * have to be atomic and may be applied in either order.
 */

-- ── 1. Columns ──────────────────────────────────────────────────────────────
-- IF NOT EXISTS: an install created after the in-place edit already has these.
ALTER TABLE flint_meta.cache_tables
    ADD COLUMN IF NOT EXISTS rls_forced   bool NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS api_granted  bool NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS policy_count int  NOT NULL DEFAULT 0;

-- ── 2. tables() ─────────────────────────────────────────────────────────────
-- The return type changes, and CREATE OR REPLACE cannot alter a function's OUT
-- parameters ("cannot change return type of existing function"), so the old one
-- must be dropped. A plain DROP fails on an extension-owned object --
--   ERROR: cannot drop function flint_meta.tables(text) because extension
--          ext-flint-meta requires it
-- so membership is released first. Verified against a live 0.1.0 install.
ALTER EXTENSION "ext-flint-meta" DROP FUNCTION flint_meta.tables(text);
DROP FUNCTION IF EXISTS flint_meta.tables(text);

-- Kept byte-identical to the definition in src/functions.rs so a fresh install
-- and an upgraded one are indistinguishable.
CREATE FUNCTION flint_meta.tables(schema_filter text DEFAULT NULL)
RETURNS TABLE (
    schema_name  text,
    table_name   text,
    is_view      bool,
    description  text,
    rls_enabled  bool,
    rls_forced   bool,
    api_granted  bool,
    policy_count int
)
LANGUAGE sql
STABLE PARALLEL SAFE
SECURITY INVOKER
AS $$
    SELECT schema_name, table_name, is_view, description, rls_enabled,
           rls_forced, api_granted, policy_count
    FROM   flint_meta.cache_tables
    WHERE  schema_filter IS NULL OR schema_name = schema_filter
    ORDER  BY schema_name, table_name;
$$;

-- DROP discarded the old grants along with the function.
GRANT EXECUTE ON FUNCTION flint_meta.tables(text) TO authenticated, anon, service_role;

-- Re-attach to the extension, but only if it is not already a member: inside a
-- real `ALTER EXTENSION ... UPDATE` Postgres auto-attaches objects created by
-- the script, and an unconditional ADD would then fail with
--   ERROR: function flint_meta.tables(text) is already a member of extension
-- Running this file by hand (psql \i) gets no auto-attach, so the ADD is needed
-- there. The guard makes both paths work.
DO $do$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM   pg_depend d
        JOIN   pg_extension e ON e.oid = d.refobjid
        WHERE  d.classid    = 'pg_proc'::regclass
          AND  d.objid      = 'flint_meta.tables(text)'::regprocedure
          AND  d.refclassid = 'pg_extension'::regclass
          AND  d.deptype    = 'e'
          AND  e.extname    = 'ext-flint-meta'
    ) THEN
        ALTER EXTENSION "ext-flint-meta" ADD FUNCTION flint_meta.tables(text);
    END IF;
END
$do$;

-- ── 3. Event trigger ────────────────────────────────────────────────────────
-- Policy and grant DDL change a table's exposure without touching the table
-- itself, so without these tags policy_count and api_granted go stale the
-- moment anyone runs CREATE POLICY or GRANT. Event triggers have no
-- CREATE OR REPLACE, so this is a drop and recreate; the tag list is kept in
-- sync with src/triggers.rs. Membership is released first for the same reason
-- as tables() above -- the trigger is extension-owned and cannot be dropped
-- while it is.
ALTER EXTENSION "ext-flint-meta" DROP EVENT TRIGGER flint_meta_ddl_refresh;
DROP EVENT TRIGGER IF EXISTS flint_meta_ddl_refresh;

CREATE EVENT TRIGGER flint_meta_ddl_refresh
    ON ddl_command_end
    WHEN TAG IN (
        'CREATE TABLE', 'ALTER TABLE',
        'CREATE VIEW',  'ALTER VIEW',
        'CREATE FUNCTION',
        'CREATE TYPE',
        'CREATE POLICY', 'ALTER POLICY', 'DROP POLICY',
        'GRANT', 'REVOKE'
    )
    EXECUTE FUNCTION flint_meta.refresh_cache();

DO $do$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM   pg_depend d
        JOIN   pg_extension e ON e.oid = d.refobjid
        JOIN   pg_event_trigger t ON t.oid = d.objid
        WHERE  d.classid    = 'pg_event_trigger'::regclass
          AND  t.evtname    = 'flint_meta_ddl_refresh'
          AND  d.refclassid = 'pg_extension'::regclass
          AND  d.deptype    = 'e'
          AND  e.extname    = 'ext-flint-meta'
    ) THEN
        ALTER EXTENSION "ext-flint-meta" ADD EVENT TRIGGER flint_meta_ddl_refresh;
    END IF;
END
$do$;

-- ── 3b. Cache-maintenance functions ─────────────────────────────────────────
-- refresh_cache(), invalidate_cache() and full_refresh() are defined in an
-- `extension_sql!` block,
-- which -- like the bootstrap file -- runs ONLY on CREATE EXTENSION. An
-- ALTER EXTENSION ... UPDATE therefore leaves both at their 0.1.0 bodies, which
-- do not write rls_forced / api_granted / policy_count at all.
--
-- Without this section the upgrade backfills posture correctly exactly once and
-- then goes stale on the very next DDL event: verified on a live install, where
-- after upgrading, `CREATE POLICY` on a FORCE-RLS table still reported
-- rls_forced=false, policy_count=0 -- the precise staleness these columns exist
-- to eliminate. The trigger fires (its tag list is updated above); the function
-- it calls was simply the old one.
--
-- All three bodies below are copied verbatim from src/triggers.rs and must be kept
-- in sync with it; that file remains the source of truth for a fresh install.

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
    -- Schema owning the table a policy DDL touched, parsed from object_identity
    -- because obj.schema_name is NULL for these tags. NULL ⇒ scope unknown
    -- (GRANT/REVOKE), which re-derives every cached table.
    v_policy_schema text;
    skip_schemas text[] := ARRAY['flint_meta', 'vault', 'pg_catalog',
                                  'information_schema', 'pg_toast'];
BEGIN
    FOR obj IN SELECT * FROM pg_event_trigger_ddl_commands() LOOP
        -- Skip internal schema DDL to avoid self-invalidation loops.
        CONTINUE WHEN obj.schema_name = ANY(skip_schemas);
        CONTINUE WHEN obj.object_identity ILIKE '%ext-flint-meta%';

        IF obj.command_tag IN ('CREATE TABLE', 'ALTER TABLE') THEN
            INSERT INTO flint_meta.cache_tables
                        (schema_name, table_name, is_view, rls_enabled, rls_forced,
                         api_granted, policy_count, updated_at)
            SELECT n.nspname,
                   c.relname,
                   c.relkind = 'v',
                   c.relrowsecurity,
                   c.relforcerowsecurity,
                   -- Reachable through the Data API at all? A table with RLS off
                   -- but no grants to the API roles is correctly hidden and must
                   -- not be reported as exposed.
                   EXISTS (SELECT 1 FROM pg_roles r
                    WHERE r.rolname IN ('authenticated','anon')
                      AND has_table_privilege(r.oid, c.oid,
                            'SELECT, INSERT, UPDATE, DELETE')),
                   (SELECT count(*) FROM pg_policy pol WHERE pol.polrelid = c.oid),
                   now()
            FROM   pg_class     c
            JOIN   pg_namespace n ON n.oid = c.relnamespace
            WHERE  n.nspname = obj.schema_name
              AND  c.relname = split_part(obj.object_identity, '.', 2)
            ON CONFLICT (schema_name, table_name) DO UPDATE
              SET rls_enabled  = EXCLUDED.rls_enabled,
                  rls_forced   = EXCLUDED.rls_forced,
                  api_granted  = EXCLUDED.api_granted,
                  policy_count = EXCLUDED.policy_count,
                  updated_at   = now();

            -- Incrementally mirror column metadata for the affected table.
            DELETE FROM flint_meta.cache_columns
            WHERE  schema_name = obj.schema_name
              AND  table_name  = split_part(obj.object_identity, '.', 2);

            INSERT INTO flint_meta.cache_columns
                        (schema_name, table_name, column_name, data_type, column_default,
                         is_nullable, is_pk, is_fk, ordinal)
            SELECT n.nspname,
                   c.relname,
                   a.attname,
                   pg_catalog.format_type(a.atttypid, a.atttypmod),
                   pg_catalog.pg_get_expr(d.adbin, d.adrelid),
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
            LEFT JOIN pg_attrdef d ON d.adrelid = c.oid AND d.adnum = a.attnum
            WHERE  n.nspname = obj.schema_name
              AND  c.relname = split_part(obj.object_identity, '.', 2)
              AND  a.attnum > 0
              AND  NOT a.attisdropped;

        ELSIF obj.command_tag IN ('CREATE POLICY', 'ALTER POLICY', 'DROP POLICY',
                                  'GRANT', 'REVOKE') THEN
            -- "<polname> on <schema>.<table>" → <schema>. substring() yields
            -- NULL when the identity has no such shape (GRANT), which the UPDATE
            -- below reads as "scope unknown, re-derive everything".
            v_policy_schema := substring(obj.object_identity from ' on ([^.]+)\.');

            -- Policy and grant DDL do not carry a usable table identity in
            -- `object_identity` (a policy identity is "pol ON schema.table";
            -- GRANT may name several objects at once), so rather than parse it,
            -- re-derive the three exposure columns for every cached table in the
            -- affected schema. Bounded by that schema's table count, and these
            -- commands are rare relative to DML.
            --
            -- The schema CANNOT be taken from obj.schema_name: for CREATE POLICY
            -- (and GRANT/REVOKE) pg_event_trigger_ddl_commands() reports
            -- schema_name = NULL, so `n.nspname = obj.schema_name` matched zero
            -- rows and policy_count silently never updated -- verified on
            -- Postgres 18. Derive it from object_identity instead, which for a
            -- policy is "<polname> on <schema>.<table>", and fall back to
            -- re-deriving every cached table when even that is unavailable (the
            -- GRANT case, where one command may span several schemas).
            UPDATE flint_meta.cache_tables ct
            SET    rls_enabled  = c.relrowsecurity,
                   rls_forced   = c.relforcerowsecurity,
                   api_granted  = EXISTS (SELECT 1 FROM pg_roles r
                    WHERE r.rolname IN ('authenticated','anon')
                      AND has_table_privilege(r.oid, c.oid,
                            'SELECT, INSERT, UPDATE, DELETE')),
                   policy_count = (SELECT count(*) FROM pg_policy pol
                                   WHERE pol.polrelid = c.oid),
                   updated_at   = now()
            FROM   pg_class     c
            JOIN   pg_namespace n ON n.oid = c.relnamespace
            WHERE  ct.schema_name = n.nspname
              AND  ct.table_name  = c.relname
              AND  n.nspname <> ALL(skip_schemas)
              AND  (v_policy_schema IS NULL OR n.nspname = v_policy_schema);

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

            DELETE FROM flint_meta.cache_columns
            WHERE  schema_name = obj.schema_name
              AND  table_name  = split_part(obj.object_identity, '.', 2);

            INSERT INTO flint_meta.cache_columns
                        (schema_name, table_name, column_name, data_type, column_default,
                         is_nullable, is_pk, is_fk, ordinal)
            SELECT n.nspname,
                   c.relname,
                   a.attname,
                   pg_catalog.format_type(a.atttypid, a.atttypmod),
                   NULL,
                   NOT a.attnotnull,
                   false,
                   false,
                   a.attnum
            FROM   pg_attribute a
            JOIN   pg_class     c ON c.oid = a.attrelid
            JOIN   pg_namespace n ON n.oid = c.relnamespace
            WHERE  n.nspname = obj.schema_name
              AND  c.relname = split_part(obj.object_identity, '.', 2)
              AND  a.attnum > 0
              AND  NOT a.attisdropped;

        ELSIF obj.command_tag IN ('CREATE FUNCTION',
                                   'CREATE OR REPLACE FUNCTION') THEN
            -- object_identity for a function includes its signature:
            -- schema.function_name(arg_type, ...). Strip the schema and args.
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
              AND  p.proname = split_part(split_part(obj.object_identity, '.', 2), '(', 1)
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
        ELSIF obj.object_type = 'policy' THEN
            -- DROP POLICY arrives here (sql_drop), NOT at refresh_cache's
            -- ddl_command_end, so without this branch policy_count only ever
            -- grew: verified on Postgres 18, where dropping one of two policies
            -- left the cache reporting 2. The dropped policy is already gone
            -- from pg_policy by the time this fires, so a plain re-count is
            -- correct. object_identity is "<polname> on <schema>.<table>";
            -- obj.schema_name is the POLICY's schema, which for a policy is the
            -- table's schema, but object_name is the policy name -- so the table
            -- has to come from the identity.
            UPDATE flint_meta.cache_tables ct
            SET    policy_count = (SELECT count(*) FROM pg_policy pol
                                   WHERE pol.polrelid = c.oid),
                   updated_at   = now()
            FROM   pg_class     c
            JOIN   pg_namespace n ON n.oid = c.relnamespace
            WHERE  ct.schema_name = n.nspname
              AND  ct.table_name  = c.relname
              AND  n.nspname = substring(obj.object_identity from ' on ([^.]+)\.')
              AND  c.relname = substring(obj.object_identity from ' on [^.]+\.(.+)$');
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
                (schema_name, table_name, is_view, rls_enabled, rls_forced,
                 api_granted, policy_count, updated_at)
    SELECT n.nspname,
           c.relname,
           c.relkind = 'v',
           c.relrowsecurity,
           c.relforcerowsecurity,
           EXISTS (SELECT 1 FROM pg_roles r
                    WHERE r.rolname IN ('authenticated','anon')
                      AND has_table_privilege(r.oid, c.oid,
                            'SELECT, INSERT, UPDATE, DELETE')),
           (SELECT count(*) FROM pg_policy pol WHERE pol.polrelid = c.oid),
           now()
    FROM   pg_class     c
    JOIN   pg_namespace n ON n.oid = c.relnamespace
    WHERE  c.relkind IN ('r', 'v', 'm')
      AND  n.nspname <> ALL(skip_schemas)
      AND  NOT n.nspname LIKE 'pg_%';

    -- Repopulate cache_columns.
    INSERT INTO flint_meta.cache_columns
                (schema_name, table_name, column_name, data_type, column_default,
                 is_nullable, is_pk, is_fk, ordinal)
    SELECT n.nspname,
           c.relname,
           a.attname,
           pg_catalog.format_type(a.atttypid, a.atttypmod),
           pg_catalog.pg_get_expr(d.adbin, d.adrelid),
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
    LEFT JOIN pg_attrdef d ON d.adrelid = c.oid AND d.adnum = a.attnum
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

-- ── 4. Backfill ─────────────────────────────────────────────────────────────
-- The columns above landed with DEFAULT false / 0 on every existing row, so
-- without a backfill every table reports as unprotected and unexposed until
-- some later DDL event happens to touch it.
--
-- Deliberately NOT `SELECT flint_meta.full_refresh()`. That function is defined
-- in an `extension_sql!` block, which -- exactly like the bootstrap file -- runs
-- only on CREATE EXTENSION. During ALTER EXTENSION ... UPDATE the *0.1.0* body
-- is still installed, and it does not know about these three columns: calling it
-- truncates cache_tables and repopulates it with the posture columns left at
-- their defaults. Verified on a live 0.1.0 install, where that path reported
-- four FORCE-RLS tables (flint_a2ui.components, .events, .component_overrides,
-- flint_kiln.cedar_policies) as rls_forced=false with policy_count=0 -- i.e.
-- silently worse than not backfilling, because the values look authoritative.
--
-- So update the posture columns in place, from pg_catalog, using the same
-- expressions full_refresh() uses. No TRUNCATE: the other cache_* tables are
-- untouched by this upgrade and their rows stay valid.
UPDATE flint_meta.cache_tables ct
SET    rls_enabled  = c.relrowsecurity,
       rls_forced   = c.relforcerowsecurity,
       api_granted  = EXISTS (SELECT 1 FROM pg_roles r
                               WHERE r.rolname IN ('authenticated','anon')
                                 AND has_table_privilege(r.oid, c.oid,
                                       'SELECT, INSERT, UPDATE, DELETE')),
       policy_count = (SELECT count(*) FROM pg_policy pol
                        WHERE pol.polrelid = c.oid),
       updated_at   = now()
FROM   pg_class c
JOIN   pg_namespace n ON n.oid = c.relnamespace
WHERE  c.relname  = ct.table_name
  AND  n.nspname  = ct.schema_name
  AND  c.relkind IN ('r', 'v', 'm');
