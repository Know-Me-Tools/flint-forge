# p5-c004 Tasks — Embeddings Pipeline

## Tasks

- [ ] Resolve OQ-10: confirm `text-embedding-3-large` is available via liter-llm gateway config in `ext-flint-llm`
- [ ] Add `a2ui-embedder` background task to `fdb-gateway` startup (PgListener on `'a2ui_embed'` channel)
- [ ] Add `pg_notify('a2ui_embed', id::text)` to `flint_a2ui.components` INSERT trigger
- [ ] Implement embedding generation: concatenate slug + description + prop names → call liter-llm → INSERT into `flint_a2ui.embeddings`
- [ ] Add `flint_a2ui.hybrid_search()` function to migration
- [ ] Initial backfill: on first startup, embed all components that lack an embedding record
- [ ] Gate test: after p5-c002 seed + embedder run, semantic search for "sortable table with pagination" returns `data-grid` as top result
- [ ] Gate test: hybrid_search for "date selection field" returns `date-picker`
