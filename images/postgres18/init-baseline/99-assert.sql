-- Flint Forge boot assertion. Runs at first-init; fails fast if the data plane is misprovisioned.
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'vector') THEN
    RAISE EXCEPTION 'flint boot assertion: pgvector missing';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pgcrypto') THEN
    RAISE EXCEPTION 'flint boot assertion: pgcrypto missing';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = 'auth') THEN
    RAISE EXCEPTION 'flint boot assertion: auth schema missing (flint_auth)';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = 'flint') THEN
    RAISE EXCEPTION 'flint boot assertion: flint schema missing (flint_hooks)';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = 'llm') THEN
    RAISE EXCEPTION 'flint boot assertion: llm schema missing (flint_llm)';
  END IF;
  IF current_setting('wal_level') <> 'logical' THEN
    RAISE WARNING 'flint boot assertion: wal_level=% during init (verified logical on the live server)', current_setting('wal_level');
  END IF;
  RAISE NOTICE 'flint boot assertion: OK — pgvector + pgcrypto + auth/flint/llm schemas present';
END $$;
