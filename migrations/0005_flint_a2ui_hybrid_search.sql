-- Migration 0005: A2UI hybrid full-text + vector search
-- Depends on: 0002_flint_a2ui (embeddings table), 0004_flint_a2ui_sdk_extensions
-- Idempotent: CREATE OR REPLACE FUNCTION

-- ── hybrid_search ───────────────────────────────────────────────────────────
-- Combines pgvector cosine similarity with BM25-style full-text ranking over
-- slug + description. Weights are configurable per call; defaults favor vector.
-- Requires: flint_a2ui.embeddings populated (p5-c004 embedder) and a GIN index
-- on components.description + components.slug (created below).

CREATE INDEX IF NOT EXISTS idx_components_fts
    ON flint_a2ui.components
    USING gin (to_tsvector('english', COALESCE(description, '') || ' ' || COALESCE(slug, '')));

CREATE OR REPLACE FUNCTION flint_a2ui.hybrid_search(
    query_text      text,
    query_embedding vector(1536),
    result_limit    int DEFAULT 10,
    vector_weight   float DEFAULT 0.7,
    text_weight     float DEFAULT 0.3
) RETURNS TABLE (
    component_id    uuid,
    slug            text,
    score           float
) LANGUAGE sql STABLE AS $$
    SELECT
        c.id,
        c.slug,
        (vector_weight * (1 - (e.embedding <=> query_embedding)) +
         text_weight * ts_rank(
             to_tsvector('english', COALESCE(c.description, '') || ' ' || COALESCE(c.slug, '')),
             plainto_tsquery('english', query_text)
        )) AS score
    FROM flint_a2ui.embeddings e
    JOIN flint_a2ui.components c ON c.id = e.component_id
    WHERE e.aspect = 'description'
    ORDER BY score DESC
    LIMIT result_limit;
$$;

COMMENT ON FUNCTION flint_a2ui.hybrid_search(text, vector(1536), int, float, float) IS
    'Hybrid full-text + vector search over the A2UI component catalog.';

GRANT EXECUTE ON FUNCTION flint_a2ui.hybrid_search(text, vector(1536), int, float, float)
    TO authenticated, service_role;
