CREATE SCHEMA IF NOT EXISTS llm;

CREATE TABLE IF NOT EXISTS llm.jobs (
  id bigserial PRIMARY KEY,
  kind text NOT NULL,                 -- 'embed' | 'summarize' | 'classify'
  schema_name text, table_name text, pk jsonb,
  source jsonb, target_column text, model text,
  dimensions int NOT NULL DEFAULT 1536,
  origin_jwt text,                    -- captured at enqueue: attribution + Cedar
  status text NOT NULL DEFAULT 'pending',
  visible_at timestamptz NOT NULL DEFAULT now(),
  retry_count int NOT NULL DEFAULT 0
);
-- dequeue pattern: SELECT ... FROM llm.jobs WHERE status='pending' AND visible_at<=now()
--                  ORDER BY id FOR UPDATE SKIP LOCKED LIMIT $batch;

-- Registry of declarative embedding configurations.
CREATE TABLE IF NOT EXISTS llm.embedding_configs (
    id bigserial PRIMARY KEY,
    schema_name text NOT NULL,
    table_name text NOT NULL,
    source_column text NOT NULL,
    target_column text NOT NULL,
    model text NOT NULL DEFAULT 'text-embedding-3-small',
    dimensions int NOT NULL DEFAULT 1536,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (schema_name, table_name, source_column, target_column)
);

-- Registry of declarative summary configurations.
CREATE TABLE IF NOT EXISTS llm.summary_configs (
    id bigserial PRIMARY KEY,
    schema_name text NOT NULL,
    table_name text NOT NULL,
    source_column text NOT NULL,
    target_column text NOT NULL,
    prompt_template text NOT NULL,
    model text NOT NULL DEFAULT 'gpt-4.1-nano',
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (schema_name, table_name, source_column, target_column)
);

-- Surface 1 (sync) signatures — provided by the extension at load:
--   llm.embed(input text, model text DEFAULT 'default') RETURNS vector
--   llm.complete(prompt text, opts jsonb DEFAULT '{}')   RETURNS text
-- Surface 2 (async) declarative provisioners:
--   llm.enable_embedding(table regclass, column text, model text, dim int)
--   llm.enable_summary(table regclass, source_col text, target_col text, prompt_template text)

-- Internal helper: primary-key column names for a relation.
CREATE OR REPLACE FUNCTION llm._pk_columns(p_rel regclass)
RETURNS text[]
LANGUAGE sql
STABLE
AS $$
    SELECT COALESCE(
        array_agg(a.attname ORDER BY array_position(i.indkey::int[], a.attnum::int)),
        '{}'::text[]
    )
    FROM pg_index i
    JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
    WHERE i.indrelid = p_rel AND i.indisprimary;
$$;

-- Generic async LLM job enqueue. Called by triggers and provisioners.
CREATE OR REPLACE FUNCTION llm.enqueue_job(
    kind text,
    schema_name text,
    table_name text,
    pk jsonb,
    source jsonb,
    target_column text,
    model text,
    dimensions int,
    origin_jwt text
) RETURNS bigint
LANGUAGE sql
AS $$
    INSERT INTO llm.jobs (kind, schema_name, table_name, pk, source, target_column, model, dimensions, origin_jwt)
    VALUES (kind, schema_name, table_name, pk, source, target_column, model, dimensions, origin_jwt)
    RETURNING id;
$$;

-- Legacy alias kept for backwards compatibility with existing triggers.
CREATE OR REPLACE FUNCTION llm.enqueue_embed(
    kind text,
    schema_name text,
    table_name text,
    pk jsonb,
    source jsonb,
    target_column text,
    model text,
    dimensions int,
    origin_jwt text
) RETURNS bigint
LANGUAGE sql
AS $$
    SELECT llm.enqueue_job(kind, schema_name, table_name, pk, source, target_column, model, dimensions, origin_jwt);
$$;

-- Write an embedding vector back to a user table row. Handles composite PKs by
-- looking up each PK column's type in pg_attribute.
CREATE OR REPLACE FUNCTION llm.writeback_vector(
    p_schema text,
    p_table text,
    p_pk jsonb,
    p_column text,
    p_vector text,
    p_dim int
) RETURNS void
LANGUAGE plpgsql
AS $$
DECLARE
    key text;
    pk_type text;
    pk_val text;
    conds text[] := '{}';
    rel regclass;
BEGIN
    rel := format('%I.%I', p_schema, p_table)::regclass;

    IF p_pk IS NULL OR jsonb_typeof(p_pk) != 'object' THEN
        RAISE EXCEPTION 'writeback_vector requires a JSONB object pk';
    END IF;

    FOR key IN SELECT jsonb_object_keys(p_pk) LOOP
        SELECT format_type(a.atttypid, a.atttypmod)
        INTO pk_type
        FROM pg_attribute a
        WHERE a.attrelid = rel AND a.attname = key;

        IF pk_type IS NULL THEN
            RAISE EXCEPTION 'primary key column % not found on %.%', key, p_schema, p_table;
        END IF;

        pk_val := p_pk->>key;
        conds := array_append(conds, format('%I = (%L)::%s', key, pk_val, pk_type));
    END LOOP;

    IF array_length(conds, 1) IS NULL THEN
        RAISE EXCEPTION 'table %.% has no primary key; cannot write back vector', p_schema, p_table;
    END IF;

    EXECUTE format(
        'UPDATE %I.%I SET %I = %L::vector(%s) WHERE %s',
        p_schema, p_table, p_column, p_vector, p_dim, array_to_string(conds, ' AND ')
    );
END;
$$;

-- Write a plain-text result back to a user table row.
CREATE OR REPLACE FUNCTION llm.writeback_text(
    p_schema text,
    p_table text,
    p_pk jsonb,
    p_column text,
    p_text text
) RETURNS void
LANGUAGE plpgsql
AS $$
DECLARE
    key text;
    pk_type text;
    pk_val text;
    conds text[] := '{}';
    rel regclass;
BEGIN
    rel := format('%I.%I', p_schema, p_table)::regclass;

    IF p_pk IS NULL OR jsonb_typeof(p_pk) != 'object' THEN
        RAISE EXCEPTION 'writeback_text requires a JSONB object pk';
    END IF;

    FOR key IN SELECT jsonb_object_keys(p_pk) LOOP
        SELECT format_type(a.atttypid, a.atttypmod)
        INTO pk_type
        FROM pg_attribute a
        WHERE a.attrelid = rel AND a.attname = key;

        IF pk_type IS NULL THEN
            RAISE EXCEPTION 'primary key column % not found on %.%', key, p_schema, p_table;
        END IF;

        pk_val := p_pk->>key;
        conds := array_append(conds, format('%I = (%L)::%s', key, pk_val, pk_type));
    END LOOP;

    IF array_length(conds, 1) IS NULL THEN
        RAISE EXCEPTION 'table %.% has no primary key; cannot write back text', p_schema, p_table;
    END IF;

    EXECUTE format(
        'UPDATE %I.%I SET %I = %L WHERE %s',
        p_schema, p_table, p_column, p_text, array_to_string(conds, ' AND ')
    );
END;
$$;

-- Generic trigger function for embedding enqueue. TG_ARGV is:
--   0: source_col, 1: target_col, 2: model, 3: dimensions
CREATE OR REPLACE FUNCTION llm._tg_enqueue_embed()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
    source_col text := TG_ARGV[0];
    target_col text := TG_ARGV[1];
    model text := TG_ARGV[2];
    dim int := TG_ARGV[3]::int;
    pk jsonb;
    src jsonb;
    jwt text;
BEGIN
    IF TG_OP = 'UPDATE' THEN
        IF to_jsonb(NEW)->target_col IS NOT NULL
           AND to_jsonb(NEW)->source_col = to_jsonb(OLD)->source_col THEN
            RETURN NEW;
        END IF;
    END IF;

    IF to_jsonb(NEW)->source_col IS NULL THEN
        RETURN NEW;
    END IF;

    IF array_length(llm._pk_columns(TG_RELID), 1) IS NULL THEN
        RAISE EXCEPTION 'llm.enable_embedding requires a primary key on %.%', TG_TABLE_SCHEMA, TG_TABLE_NAME;
    END IF;

    SELECT jsonb_object_agg(a.attname, to_jsonb(NEW)->a.attname)
    INTO pk
    FROM unnest(llm._pk_columns(TG_RELID)) a(attname);

    src := jsonb_build_object('text', to_jsonb(NEW)->>source_col);
    jwt := current_setting('request.jwt', true);

    PERFORM llm.enqueue_job('embed', TG_TABLE_SCHEMA, TG_TABLE_NAME, pk, src, target_col, model, dim, jwt);
    RETURN NEW;
END;
$$;

-- Generic trigger function for summary enqueue. TG_ARGV is:
--   0: source_col, 1: target_col, 2: prompt_template, 3: model
CREATE OR REPLACE FUNCTION llm._tg_enqueue_summary()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
    source_col text := TG_ARGV[0];
    target_col text := TG_ARGV[1];
    prompt_template text := TG_ARGV[2];
    model text := TG_ARGV[3];
    pk jsonb;
    prompt text;
    src jsonb;
    jwt text;
BEGIN
    IF TG_OP = 'UPDATE' THEN
        IF to_jsonb(NEW)->target_col IS NOT NULL
           AND to_jsonb(NEW)->source_col = to_jsonb(OLD)->source_col THEN
            RETURN NEW;
        END IF;
    END IF;

    IF to_jsonb(NEW)->source_col IS NULL THEN
        RETURN NEW;
    END IF;

    IF array_length(llm._pk_columns(TG_RELID), 1) IS NULL THEN
        RAISE EXCEPTION 'llm.enable_summary requires a primary key on %.%', TG_TABLE_SCHEMA, TG_TABLE_NAME;
    END IF;

    SELECT jsonb_object_agg(a.attname, to_jsonb(NEW)->a.attname)
    INTO pk
    FROM unnest(llm._pk_columns(TG_RELID)) a(attname);

    prompt := llm._render_template(prompt_template, to_jsonb(NEW));
    src := jsonb_build_object('text', to_jsonb(NEW)->>source_col, 'prompt', prompt);
    jwt := current_setting('request.jwt', true);

    PERFORM llm.enqueue_job('summarize', TG_TABLE_SCHEMA, TG_TABLE_NAME, pk, src, target_col, model, 0, jwt);
    RETURN NEW;
END;
$$;

-- Declarative provisioner: keep a target vector column in sync with a source text column.
CREATE OR REPLACE FUNCTION llm.enable_embedding(
    target_table regclass,
    source_col text,
    target_col text,
    model text DEFAULT 'text-embedding-3-small',
    dimensions int DEFAULT 1536
) RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
    schema_name text;
    table_name text;
    trig_name text;
    has_col bool;
BEGIN
    SELECT n.nspname, c.relname INTO schema_name, table_name
    FROM pg_class c
    JOIN pg_namespace n ON n.oid = c.relnamespace
    WHERE c.oid = target_table;

    IF schema_name IS NULL THEN
        RAISE EXCEPTION 'table % not found', target_table;
    END IF;

    -- Add target vector column if absent.
    SELECT EXISTS (
        SELECT 1 FROM pg_attribute a
        WHERE a.attrelid = target_table AND a.attname = target_col AND NOT a.attisdropped
    ) INTO has_col;
    IF NOT has_col THEN
        EXECUTE format('ALTER TABLE %I.%I ADD COLUMN %I vector(%s)', schema_name, table_name, target_col, dimensions);
    END IF;

    -- Add HNSW index if absent.
    IF NOT EXISTS (
        SELECT 1 FROM pg_indexes
        WHERE schemaname = schema_name AND tablename = table_name
          AND indexname = 'flint_embed_' || schema_name || '_' || table_name || '_' || target_col || '_idx'
    ) THEN
        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS %I ON %I.%I USING hnsw (%I vector_cosine_ops)',
            'flint_embed_' || schema_name || '_' || table_name || '_' || target_col || '_idx',
            schema_name, table_name, target_col
        );
    END IF;

    -- Record configuration.
    INSERT INTO llm.embedding_configs (schema_name, table_name, source_column, target_column, model, dimensions)
    VALUES (schema_name, table_name, source_col, target_col, model, dimensions)
    ON CONFLICT (schema_name, table_name, source_column, target_column)
    DO UPDATE SET model = EXCLUDED.model, dimensions = EXCLUDED.dimensions;

    trig_name := 'flint_embed_' || schema_name || '_' || table_name || '_' || target_col;

    EXECUTE format(
        'DROP TRIGGER IF EXISTS %I ON %I.%I',
        trig_name, schema_name, table_name
    );
    EXECUTE format(
        'CREATE TRIGGER %I AFTER INSERT OR UPDATE ON %I.%I FOR EACH ROW EXECUTE FUNCTION llm._tg_enqueue_embed(%L, %L, %L, %L)',
        trig_name, schema_name, table_name, source_col, target_col, model, dimensions
    );

    -- Backfill existing rows that have source but no embedding.
    EXECUTE format(
        'INSERT INTO llm.jobs (kind, schema_name, table_name, pk, source, target_column, model, dimensions, visible_at)
         SELECT ''embed'', %L, %L,
                jsonb_object_agg(a.attname, to_jsonb(t)->a.attname),
                jsonb_build_object(''text'', to_jsonb(t)->>%L),
                %L, %L, %L,
                now() + (row_number() OVER () * interval ''100 ms'')
         FROM %I.%I t
         CROSS JOIN LATERAL unnest(llm._pk_columns(%L::regclass)) a(attname)
         WHERE t.%I IS NOT NULL AND t.%I IS NULL
         GROUP BY t.ctid',
        schema_name, table_name, source_col, target_col, model, dimensions,
        schema_name, table_name, schema_name || '.' || table_name, source_col, target_col
    );
END;
$$;

-- Declarative provisioner: keep a target text column in sync with a source text column.
CREATE OR REPLACE FUNCTION llm.enable_summary(
    target_table regclass,
    source_col text,
    target_col text,
    prompt_template text,
    model text DEFAULT 'gpt-4.1-nano'
) RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
    schema_name text;
    table_name text;
    trig_name text;
    has_col bool;
BEGIN
    SELECT n.nspname, c.relname INTO schema_name, table_name
    FROM pg_class c
    JOIN pg_namespace n ON n.oid = c.relnamespace
    WHERE c.oid = target_table;

    IF schema_name IS NULL THEN
        RAISE EXCEPTION 'table % not found', target_table;
    END IF;

    -- Add target text column if absent.
    SELECT EXISTS (
        SELECT 1 FROM pg_attribute a
        WHERE a.attrelid = target_table AND a.attname = target_col AND NOT a.attisdropped
    ) INTO has_col;
    IF NOT has_col THEN
        EXECUTE format('ALTER TABLE %I.%I ADD COLUMN %I text', schema_name, table_name, target_col);
    END IF;

    -- Record configuration.
    INSERT INTO llm.summary_configs (schema_name, table_name, source_column, target_column, prompt_template, model)
    VALUES (schema_name, table_name, source_col, target_col, prompt_template, model)
    ON CONFLICT (schema_name, table_name, source_column, target_column)
    DO UPDATE SET prompt_template = EXCLUDED.prompt_template, model = EXCLUDED.model;

    trig_name := 'flint_summary_' || schema_name || '_' || table_name || '_' || target_col;

    EXECUTE format(
        'DROP TRIGGER IF EXISTS %I ON %I.%I',
        trig_name, schema_name, table_name
    );
    EXECUTE format(
        'CREATE TRIGGER %I AFTER INSERT OR UPDATE ON %I.%I FOR EACH ROW EXECUTE FUNCTION llm._tg_enqueue_summary(%L, %L, %L, %L)',
        trig_name, schema_name, table_name, source_col, target_col, prompt_template, model
    );

    -- Backfill existing rows that have source but no summary.
    EXECUTE format(
        'INSERT INTO llm.jobs (kind, schema_name, table_name, pk, source, target_column, model, dimensions, visible_at)
         SELECT ''summarize'', %L, %L,
                jsonb_object_agg(a.attname, to_jsonb(t)->a.attname),
                jsonb_build_object(
                    ''text'', to_jsonb(t)->>%L,
                    ''prompt'', llm._render_template(%L, to_jsonb(t))
                ),
                %L, %L, 0,
                now() + (row_number() OVER () * interval ''100 ms'')
         FROM %I.%I t
         CROSS JOIN LATERAL unnest(llm._pk_columns(%L::regclass)) a(attname)
         WHERE t.%I IS NOT NULL AND t.%I IS NULL
         GROUP BY t.ctid',
        schema_name, table_name, source_col, prompt_template, target_col, model,
        schema_name, table_name, schema_name || '.' || table_name, source_col, target_col
    );
END;
$$;

CREATE OR REPLACE FUNCTION llm.embed(input text, model text DEFAULT 'default')
RETURNS vector
LANGUAGE sql
STABLE
AS $$
    SELECT llm._embed_text($1, $2)::vector;
$$;

CREATE OR REPLACE FUNCTION llm.complete(prompt text, opts jsonb DEFAULT '{}'::jsonb, model text DEFAULT 'default')
RETURNS text
LANGUAGE sql
STABLE
AS $$
    SELECT llm._complete($1, $3, $2);
$$;

-- Lockdown: no LLM surface is available to PUBLIC or anonymous sessions.
REVOKE ALL ON SCHEMA llm FROM PUBLIC;
REVOKE ALL ON TABLE llm.jobs FROM PUBLIC;
REVOKE ALL ON TABLE llm.embedding_configs FROM PUBLIC;
REVOKE ALL ON TABLE llm.summary_configs FROM PUBLIC;
REVOKE ALL ON FUNCTION llm._embed_text(text, text) FROM PUBLIC;
REVOKE ALL ON FUNCTION llm._complete(text, text, jsonb) FROM PUBLIC;
REVOKE ALL ON FUNCTION llm._render_template(text, jsonb) FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'authenticated') THEN
        GRANT USAGE ON SCHEMA llm TO authenticated;
        GRANT SELECT, INSERT, UPDATE ON TABLE llm.jobs TO authenticated;
        GRANT SELECT ON TABLE llm.embedding_configs TO authenticated;
        GRANT SELECT ON TABLE llm.summary_configs TO authenticated;
        GRANT EXECUTE ON FUNCTION llm.enable_embedding(regclass, text, text, text, int) TO authenticated;
        GRANT EXECUTE ON FUNCTION llm.enable_summary(regclass, text, text, text, text) TO authenticated;
        GRANT EXECUTE ON FUNCTION llm.embed(text, text) TO authenticated;
        GRANT EXECUTE ON FUNCTION llm.complete(text, jsonb, text) TO authenticated;
    END IF;
END
$$;
