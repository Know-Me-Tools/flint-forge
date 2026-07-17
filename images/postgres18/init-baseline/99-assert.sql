-- Flint Forge boot assertion. Runs at first-init; fails fast if the data plane is misprovisioned.
--
-- Two tiers of check, deliberately kept separate:
--   1. Presence  — extension/schema rows exist in pg_extension/pg_namespace.
--   2. Callable  — the reflection-critical flint_meta.* functions this image
--      ships actually EXECUTE and return the shape fdb-reflection expects.
--
-- Presence-only checks pass for a stale image: `CREATE EXTENSION` succeeds as
-- long as the .control/.sql files are present in
-- /usr/share/postgresql/18/extension/, even if those files predate a source
-- change that added or resignatured a function (see incident: a locally-built
-- image from before ext-flint-meta commit da7a24c had flint_meta.tables()/
-- functions()/columns() installed and passing presence checks, while
-- flint_meta.views() was silently absent — fdb-gateway didn't panic until
-- first request, well past this assertion). Calling each function is the only
-- check that catches a stale/partial extension build at container boot.
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'vector') THEN
    RAISE EXCEPTION 'flint boot assertion: pgvector missing';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pgcrypto') THEN
    RAISE EXCEPTION 'flint boot assertion: pgcrypto missing';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_net') THEN
    RAISE EXCEPTION 'flint boot assertion: pg_net missing';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_cron') THEN
    RAISE EXCEPTION 'flint boot assertion: pg_cron missing';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'flint_llm') THEN
    RAISE EXCEPTION 'flint boot assertion: flint_llm extension missing';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'flint_vault') THEN
    RAISE EXCEPTION 'flint boot assertion: flint_vault extension missing';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'ext-flint-meta') THEN
    RAISE EXCEPTION 'flint boot assertion: ext-flint-meta extension missing';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'ext-flint-auth') THEN
    RAISE EXCEPTION 'flint boot assertion: ext-flint-auth extension missing';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'ext-flint-hooks') THEN
    RAISE EXCEPTION 'flint boot assertion: ext-flint-hooks extension missing';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = 'auth') THEN
    RAISE EXCEPTION 'flint boot assertion: auth schema missing';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = 'flint') THEN
    RAISE EXCEPTION 'flint boot assertion: flint schema missing';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = 'llm') THEN
    RAISE EXCEPTION 'flint boot assertion: llm schema missing';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = 'vault') THEN
    RAISE EXCEPTION 'flint boot assertion: vault schema missing';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = 'flint_meta') THEN
    RAISE EXCEPTION 'flint boot assertion: flint_meta schema missing';
  END IF;
  IF current_setting('wal_level') <> 'logical' THEN
    RAISE WARNING 'flint boot assertion: wal_level=% during init (verified logical on the live server)', current_setting('wal_level');
  END IF;
  RAISE NOTICE 'flint boot assertion: presence OK — all Anvil extensions and schemas present';
END $$;

-- ── Callable checks: every flint_meta.* function fdb-reflection's
-- ReflectionEngine::reflect() (crates/fdb-reflection/src/engine.rs) calls on
-- EVERY gateway startup and EVERY schema hot-swap. Each is called for real
-- (not just checked via pg_proc) with the exact call shape fdb-gateway uses,
-- so a signature drift (e.g. a renamed/re-ordered argument with a different
-- default) fails here, not as a 42883/42703 at first request in production.
--
-- Gated on ext-flint-meta actually being installed: this same 99-assert.sql
-- file is shared with Dockerfile.baseline, which intentionally never builds
-- the Anvil pgrx extensions (pure-SQL subset only) and so has no flint_meta
-- schema at all. Skip rather than fail when the extension is absent — the
-- presence-check block above is the correct place to require it exists for
-- images that are supposed to ship it.
DO $$
DECLARE
  v_count bigint;
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'ext-flint-meta') THEN
    RAISE NOTICE 'flint boot assertion: callable checks SKIPPED — ext-flint-meta not installed on this image variant';
    RETURN;
  END IF;

  -- flint_meta.tables(schema_filter text DEFAULT NULL)
  PERFORM schema_name, table_name, is_view, description, rls_enabled
  FROM flint_meta.tables(NULL)
  LIMIT 0;

  -- flint_meta.columns(p_schema text, p_table text) — probe with a table that
  -- is always present (flint.webhooks, created by ext-flint-hooks) so a real
  -- row shape is exercised, not just an empty-result no-op. NOTE:
  -- flint_meta.cache_tables/cache_columns deliberately do NOT track the
  -- flint_meta schema's own tables (the reflection cache doesn't catalog
  -- itself), so that table is never a valid probe target here.
  SELECT count(*) INTO v_count
  FROM flint_meta.columns('flint', 'webhooks');
  IF v_count = 0 THEN
    RAISE EXCEPTION 'flint boot assertion: flint_meta.columns() returned no rows for flint.webhooks — reflection cache not populated or function broken';
  END IF;

  -- flint_meta.relationships(p_schema text, p_table text)
  PERFORM from_schema, from_table, from_column, to_schema, to_table, to_column, constraint_name
  FROM flint_meta.relationships('flint_meta', 'cache_tables')
  LIMIT 0;

  -- flint_meta.functions(p_schema text DEFAULT NULL)
  PERFORM schema_name, function_name, return_type, security_definer
  FROM flint_meta.functions(NULL)
  LIMIT 0;

  -- flint_meta.function_args(p_schema text, p_function text)
  PERFORM arg_name, arg_type
  FROM flint_meta.function_args('flint_meta', 'full_refresh')
  LIMIT 0;

  -- flint_meta.views(): the exact regression this assertion was added for —
  -- see incident note above. Must be present and return the (schema_name,
  -- view_name, security_barrier) shape fdb-reflection's fetch_views() expects.
  PERFORM schema_name, view_name, security_barrier
  FROM flint_meta.views()
  LIMIT 0;

  RAISE NOTICE 'flint boot assertion: callable OK — flint_meta.{tables,columns,relationships,functions,function_args,views}() all executed successfully';
EXCEPTION
  WHEN undefined_function THEN
    RAISE EXCEPTION 'flint boot assertion: a flint_meta reflection function is missing (%). The ext-flint-meta image was built from stale source — rebuild images/postgres18/Dockerfile from current main.', SQLERRM;
  WHEN undefined_column THEN
    RAISE EXCEPTION 'flint boot assertion: a flint_meta reflection function returned an unexpected column shape (%). This means fdb-reflection/src/engine.rs and ext-flint-meta/src/functions.rs have drifted out of sync.', SQLERRM;
END $$;
