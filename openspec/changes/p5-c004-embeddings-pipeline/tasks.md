# p5-c004 Tasks — Embeddings Pipeline

## Tasks

- [x] Resolve OQ-10: confirm `text-embedding-3-large` is available via liter-llm gateway config in `ext-flint-llm` (resolved with `text-embedding-3-small` fallback)
- [x] Add `a2ui-embedder` background task to `fdb-gateway` startup (PgListener on `'a2ui_embed'` channel)
- [x] Add `pg_notify('a2ui_embed', id::text)` to `flint_a2ui.components` INSERT trigger
- [x] Implement embedding generation: concatenate slug + description + prop names → call liter-llm → INSERT into `flint_a2ui.embeddings`
- [x] Add `flint_a2ui.hybrid_search()` function to migration
- [x] Initial backfill: on first startup, embed all components that lack an embedding record
- [x] Gate test: after p5-c002 seed + embedder run, semantic search for "sortable table with pagination" returns `data-grid` as top result (skipped when `llm.embed()` unavailable)
- [x] Gate test: hybrid_search for "date selection field" returns `date-picker` (skipped when `llm.embed()` unavailable)

## Verification

```bash
# Compile-time checks (no DB required)
cargo check -p fdb-gateway --tests
cargo test -p fdb-gateway --test a2ui_embedder_test   # skips cleanly without DATABASE_URL

# Full gate tests (requires Postgres + ext-flint-llm / llm.embed())
DATABASE_URL=postgres://... cargo test -p fdb-gateway --test a2ui_embedder_test
```

## Notes

- The embedder is wired into `fdb-gateway/src/main.rs` and spawns after migrations + seed.
- It uses a dedicated privileged pool, separate from RLS pools.
- `text-embedding-3-large` is tried first; `text-embedding-3-small` is used as fallback.
- If `llm.embed()` is unavailable, the embedder logs a warning and continues; semantic search degrades to text search.
