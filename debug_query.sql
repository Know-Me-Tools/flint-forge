CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS pg_net;
CREATE EXTENSION IF NOT EXISTS pg_cron;
CREATE EXTENSION IF NOT EXISTS flint_vault;
CREATE EXTENSION flint_llm;

SELECT proname, pronamespace::regnamespace, pg_get_function_arguments(oid)
FROM pg_proc
WHERE proname IN ('_embed_text','_complete','_render_template','llm_version');

REVOKE ALL ON FUNCTION llm._embed_text(text, text) FROM PUBLIC;
