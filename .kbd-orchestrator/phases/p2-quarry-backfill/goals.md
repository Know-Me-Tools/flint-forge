# Goals — p2-quarry-backfill

## Phase Gate

All Phase 2 P1 backfill changes are delivered and qa_passed: `pgvector` `/rpc` endpoint operational; `GET /openapi.json` returns a valid OpenAPI 3.1 document merged from the reflection-compiled `CompiledState.openapi_doc`.

## Goals

- **G1 — p2-c006-pgvector-rpc:** `/rpc` route in `fdb-gateway` accepts a vector similarity request body (`embedding: Vec<f32>`, `table`, `column`, `limit`, optional `filter`), executes `SELECT ... ORDER BY <col> <=> $1 LIMIT $2` via `PgBackend`, returns JSON rows. This unblocks Phase 5 `p5-c001` (pgvector schema extensions).

- **G2 — p2-c007-openapi-compiler:** `GET /openapi.json` returns the full OpenAPI 3.1 document already compiled by `OpenApiCompiler::compile()` in `CompiledState.openapi_doc`. The endpoint reads `state_manager.current().openapi_doc` and serialises it. No new compilation logic — the compiler is already wired in `StateManager::do_compile()` (p2-c003). This unblocks Phase 7 (MCP surface) and provides a machine-readable schema for SDK generation.

## Dependencies

### From Phase 2 (all delivered)
- `fdb-reflection`: `CompiledState.openapi_doc` (serde_json::Value) — **DELIVERED** (p2-c003)
- `fdb-postgres`: `PgBackend::acquire()` with full 6-GUC SET LOCAL block — **DELIVERED** (p2-c002 + p3-c008)
- `fdb-gateway`: composition root wired to `StateManager`, `/healthz` route — **DELIVERED** (p2-c005)
- `forge-identity`: `RlsContext` with extended fields — **DELIVERED** (p2-c001 + p3-c008)

### External
- pgvector Postgres extension must be installed in the PG18 image (OQ-9: pgvector ≥ 0.7.0)
- OQ-9 should be verified before coding p2-c006 (check `/Users/gqadonis/Projects/prometheus/flint-forge/images/postgres18/Dockerfile`)

## Phase Complete When
- `cargo test --workspace` passes
- `cargo clippy --workspace -- -D warnings` passes
- `GET /rpc` handles a valid vector similarity request (unit tested)
- `GET /openapi.json` returns the compiled OpenAPI document (unit tested)
- Both changes are `qa_passed` in `progress.json`
