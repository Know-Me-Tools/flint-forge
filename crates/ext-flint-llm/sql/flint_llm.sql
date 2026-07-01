CREATE SCHEMA IF NOT EXISTS llm;

CREATE TABLE IF NOT EXISTS llm.jobs (
  id bigserial PRIMARY KEY,
  kind text NOT NULL,                 -- 'embed' | 'summarize' | 'classify'
  schema_name text, table_name text, pk jsonb,
  source jsonb, target_column text, model text,
  origin_jwt text,                    -- captured at enqueue: attribution + Cedar
  status text NOT NULL DEFAULT 'pending',
  visible_at timestamptz NOT NULL DEFAULT now(),
  retry_count int NOT NULL DEFAULT 0
);
-- dequeue pattern: SELECT ... FROM llm.jobs WHERE status='pending' AND visible_at<=now()
--                  ORDER BY id FOR UPDATE SKIP LOCKED LIMIT $batch;

-- Surface 1 (sync) signatures — provided by the extension at load:
--   llm.embed(input text, model text DEFAULT 'default') RETURNS vector
--   llm.complete(prompt text, opts jsonb DEFAULT '{}')   RETURNS text
-- Surface 2 (async) declarative provisioners:
--   llm.enable_embedding(table regclass, column text, model text, dim int)
--   llm.enable_summary(table regclass, source_col text, target_col text, prompt_template text)
