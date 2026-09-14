\set ON_ERROR_STOP on
-- Run explicitly on both existing and fresh databases after image replacement.
-- Everything is transactional; never drop the legacy schema or its dependents.
BEGIN;
SELECT pg_advisory_xact_lock(721405001);
DO $upgrade$
DECLARE object record; version text;
BEGIN
  SELECT extversion INTO version FROM pg_extension WHERE extname='flint_llm';
  IF version='0.1.0' THEN RETURN; END IF;
  IF version IS NOT NULL AND version <> '0.0.0' THEN
    RAISE EXCEPTION 'Unsupported flint_llm version: %',version;
  END IF;
  IF version IS NULL AND to_regclass('llm.jobs') IS NULL THEN
    CREATE EXTENSION flint_llm VERSION '0.1.0';
    RETURN;
  END IF;
  IF version IS NULL THEN
    CREATE EXTENSION flint_llm VERSION '0.0.0';
  END IF;
  -- Adopt only the documented legacy objects. Existing ownership, ACLs and
  -- dependencies survive ALTER EXTENSION ADD and CREATE OR REPLACE FUNCTION.
  FOR object IN
    SELECT c.oid, format('%I.%I', n.nspname,c.relname) AS name
    FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
    WHERE n.nspname='llm' AND c.relkind='r'
      AND c.relname IN ('jobs','embedding_configs','summary_configs')
      AND NOT EXISTS (SELECT 1 FROM pg_depend d WHERE d.classid='pg_class'::regclass
                      AND d.objid=c.oid AND d.deptype='e')
  LOOP EXECUTE format('ALTER EXTENSION flint_llm ADD TABLE %s',object.name); END LOOP;
  FOR object IN
    SELECT p.oid::regprocedure AS signature
    FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace
    WHERE n.nspname='llm' AND p.proname IN
      ('_pk_columns','enqueue_job','enqueue_embed','writeback_vector','writeback_text',
       '_tg_enqueue_embed','_tg_enqueue_summary','enable_embedding','enable_summary','embed','complete')
      AND NOT EXISTS (SELECT 1 FROM pg_depend d WHERE d.classid='pg_proc'::regclass
                      AND d.objid=p.oid AND d.deptype='e')
  LOOP EXECUTE format('ALTER EXTENSION flint_llm ADD FUNCTION %s',object.signature); END LOOP;
  ALTER EXTENSION flint_llm UPDATE TO '0.1.0';
END
$upgrade$;
DO $$ BEGIN
  IF to_regprocedure('llm.embed(text,text)') IS NULL OR to_regprocedure('llm._embed_text(text,text)') IS NULL THEN
    RAISE EXCEPTION 'Ember upgrade incomplete: embedding functions missing';
  END IF;
END $$;
COMMIT;
