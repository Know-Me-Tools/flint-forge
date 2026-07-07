# Goals — p4-flint-ember

**Phase gate:** A signed component handles an HTTP request and calls back into Quarry/Ember under origin identity, RLS-enforced.

Seeded from `p3.5-ci-postgres-hardening/reflection.md` and `docs/FLINT-FORGE-SPEC.md` §4.3.

## Goals

- **G1** — `ext-flint-llm` pgrx extension exists with `flint_llm` SQL surface and liter-llm binding. All model calls route through flint-gate/UAR; no provider keys are stored in the database.
- **G2** — Async embeddings background worker + `llm.jobs` queue + `llm.enable_embedding` declarative surface; rate-limit governor prevents runaway spend.
- **G3** — Sync surface: `llm.embed()` and `llm.complete()` with interrupt/timeout safety and proper error propagation.
- **G4** — Async summaries: `llm.enable_summary()` surface using the same BGW + queue infrastructure.

## Dependencies from p3.5-ci-postgres-hardening

- `flint_meta` schema and reflection pipeline — delivered.
- CI Postgres service with `DATABASE_URL` wiring — implemented (verification pending).
- `fdb-reflection` `CompiledState` + `StateManager` hot-swap — delivered.

## Open questions carried in

- **OQ-10** — Embedding model access: is `text-embedding-3-large` available via the liter-llm gateway already configured for `ext-flint-llm`? Can use `text-embedding-3-small` (1536-d) as fallback at lower quality.
- **OQ-FRF-1** — unchanged; FRF `WatchEntityType` path stays deferred.
