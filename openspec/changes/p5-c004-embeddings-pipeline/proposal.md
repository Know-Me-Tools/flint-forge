# p5-c004 — Embeddings Pipeline

**Phase:** 5 — Flint A2UI Component Registry  
**Priority:** P1 (semantic search degrades to text search until this is done; not blocking for MVP)  
**Depends on:** p5-c001, p5-c002  
**Blocks:** p5-c006 (semantic search endpoint in REST API)

---

## What this change delivers

- Background Rust task (`a2ui-embedder`) that generates vector embeddings for all `flint_a2ui.components` lacking embeddings
- Integration with liter-llm gateway (already configured for `ext-flint-llm`) using `text-embedding-3-large` (1536 dimensions)
- Fallback to `text-embedding-3-small` (also 1536-d) if large model is unavailable (see OQ-10)
- Auto-embedding on `INSERT INTO flint_a2ui.components` via `pg_notify('a2ui_embed', component_id::text)`
- `flint_a2ui.semantic_search(query_embedding, limit)` function activated (stub installed in p5-c001)

### Embedding input format per component

For each component, embed the concatenation of:
```
{slug} {primitive_type} {category} {description}

Usage: {usage_examples as plain text}

Props: {prop names from JSON Schema properties keys}
```

This ensures agents can find components by describing what they need in natural language.

### Hybrid search function

```sql
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
         text_weight * ts_rank(to_tsvector('english', c.description || ' ' || c.slug),
                               plainto_tsquery('english', query_text))) AS score
    FROM flint_a2ui.embeddings e
    JOIN flint_a2ui.components c ON c.id = e.component_id
    WHERE e.aspect = 'description'
    ORDER BY score DESC
    LIMIT result_limit;
$$;
```

---

## Out of scope

- Multi-modal embeddings (screenshots of components) — Section 16.1 item 2 in RFC-FORGE-A2UI-001; deferred
- Cross-encoder re-ranking — deferred to Phase 5 hardening
