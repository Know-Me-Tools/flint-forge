-- Flint Forge boot assertion. Runs at first-init; fails fast if the data plane is misprovisioned.
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
  RAISE NOTICE 'flint boot assertion: OK — all Anvil extensions and schemas present';
END $$;
