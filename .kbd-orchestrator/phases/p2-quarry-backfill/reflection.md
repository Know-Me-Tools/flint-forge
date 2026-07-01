# Reflection — p2-quarry-backfill

**Date:** 2026-06-30  
**Phase:** p2-quarry-backfill  
**Status:** COMPLETE  
**Changes completed:** 2/2

---

## Goal Achievement

| Goal | Status | Notes |
|------|--------|-------|
| G1 — p2-c006-pgvector-rpc: POST /rpc/vector | MET | VectorRpcRequest domain type, PgVectorRpc adapter, /rpc/vector route — all delivered. SQL injection guard (is_safe_identifier) confirmed. 4/4 unit tests pass. |
| G2 — p2-c007-openapi-compiler: GET /openapi.json | MET | OpenApiCompiler::compile() fully implemented (was todo!() startup panic). GET /openapi.json wired. 13/13 unit tests pass. |

**Phase gate:** 100% — all MVP changes qa_passed, cargo test --workspace PASS, cargo clippy -D warnings CLEAN.

---

## Delivered Changes

### p2-c007-openapi-compiler (executed first — critical path)

**Problem solved:** `OpenApiCompiler::compile()` was a `todo!()` that panicked at startup, making the gateway completely unrunnable. `StateManager::do_compile()` calls it synchronously on first boot and on every DDL change.

**What was built:**
- Full OpenAPI 3.1.0 document generator from `DatabaseModel` in `crates/fdb-reflection/src/compilers/openapi.rs`
- Per-table collection path (`GET`/`POST`) and item path (`GET`/`PATCH`/`DELETE`) with `id` parameter
- Per-function `POST /rpc/{schema}/{name}` path with request body schema
- 12 filter operators per column (`eq`, `neq`, `gt`, `gte`, `lt`, `lte`, `like`, `ilike`, `in`, `is`, `cs`, `cd`)
- Postgres type → JSON Schema mapping (`text`/`varchar`→`string`, `int4`/`int8`/`bigint`→`integer`, `float8`/`numeric`→`number`, `bool`→`boolean`, `jsonb`→`object`, `vector`→`array`, default→`string`)
- Bearer security scheme + global `security: [bearerAuth]` requirement
- `x-rls-enabled` extension field on table schemas
- `GET /openapi.json` handler in `fdb-gateway` — reads `CompiledState.openapi_doc`, no auth required

**Files modified:** `crates/fdb-reflection/src/compilers/openapi.rs`, `crates/fdb-gateway/src/main.rs`  
**Tests added:** 13  
**Test result:** 13/13 pass

---

### p2-c006-pgvector-rpc (executed second)

**Problem solved:** No vector similarity search capability existed. Phase 5 (`p5-c001` pgvector schema extensions) is blocked without a working `/rpc/vector` endpoint.

**What was built:**
- `VectorRpcRequest` domain type in `crates/fdb-domain/src/lib.rs` (Layer 0 — zero infra deps)
- `pgvector = { version = "0.4", features = ["postgres"] }` added to workspace `Cargo.toml`
- `PgVectorRpc` adapter in `crates/fdb-postgres/src/lib.rs` with `execute_similarity()` method
- `is_safe_identifier()` SQL injection guard — alphanumeric/underscore/dot only, max 128 chars, no leading/trailing dots; rejects any injection attempts before SQL construction
- Similarity query: `SELECT *, ({col} <=> $1::vector) AS distance FROM {tbl} ORDER BY {col} <=> $1::vector LIMIT $2` under full 6-GUC RLS context via `PgBackend::acquire()`
- Limit capped at 1000 regardless of client request
- `rpc_vector_handler` in `fdb-gateway` with bearer extraction + `rls_from_bearer()` + error mapping (400 for invalid identifiers, 401 for auth failures, 500 for internal errors)

**Files modified:** `Cargo.toml`, `crates/fdb-domain/src/lib.rs`, `crates/fdb-postgres/Cargo.toml`, `crates/fdb-postgres/src/lib.rs`, `crates/fdb-gateway/src/main.rs`  
**Tests added:** 4  
**Test result:** 4/4 pass

---

## Artifact Quality Summary

| Metric | Value |
|--------|-------|
| Changes with QA | 2/2 |
| First-pass pass rate | 2/2 (100%) |
| Changes requiring refinement | 0 |
| Clippy fixes required | 1 (redundant closure in PgVectorRpc row serializer — `\|v\| serde_json::Value::String(v)` → `serde_json::Value::String`) |

*No artifact-refiner logs exist (refiner not wired for this phase). QA was conducted via direct cargo toolchain.*

### Constraint Violations: None

No BLOCK-level constraint violations encountered:
- No `unwrap()`/`expect()` in library crates
- No SQL injection (is_safe_identifier guard validated)
- No JWT/claim/tenant data in tracing spans
- No file exceeded 500 lines
- Hexagonal dependency rule maintained throughout

---

## Scope Deltas vs Plan

### Delta 1 — pgvector version resolved to 0.4.2 (not 0.7 as planned)

**Plan stated:** `pgvector = "0.7"`  
**Actual:** `pgvector = "0.4"` (resolves to 0.4.2)

**Root cause:** The pgvector Rust crate versioning does not map to the pgvector Postgres extension version. The current crate release series is 0.4.x. The Postgres extension version (0.7.4 in the PG18 image) is a separate version axis. The crate's `features = ["postgres"]` exposes `ToSql`/`FromSql` for `tokio-postgres` in both 0.4.x and a hypothetical 0.7.x.

**Impact:** None. The `pgvector = "0.4"` crate provides `pgvector::Vector` with full `ToSql` support for the Postgres `vector` type. The Dockerfile has `postgresql-18-pgvector` 0.7.4 which is the extension — the two versions are independent.

**Corrective action:** Update `openspec/changes/p2-c006-pgvector-rpc/proposal.md` to clarify the Rust crate vs Postgres extension version distinction before Phase 5 reads it.

### Delta 2 — p2-c006 MVP scope confirmed (no general-purpose RPC pgvector injection)

**Plan stated:** General-purpose pgvector arg injection into reflected function RPCs deferred.  
**Actual:** Confirmed deferred. Dedicated `/rpc/vector` endpoint delivered as planned.

**Impact:** Phase 5 `p5-c001` is unblocked. General-purpose pgvector RPC injection remains a follow-on to `p2-c004` (REST compiler query builder, not yet delivered).

### Delta 3 — Row serializer uses text format for all column types

**Plan stated:** Returns JSON rows.  
**Actual:** The `execute_similarity()` serializer uses `row.try_get::<_, Option<String>>(i)` for every column. Non-string columns (integers, booleans, JSON) serialize as their `text` Postgres representation rather than typed JSON values.

**Root cause:** `tokio-postgres` requires type-specific `try_get` calls; generic deserialization to `serde_json::Value` without matching against column types would require either `tokio-postgres`'s `serde` feature or a type-switch on `col.type_()`. The text-format approach was chosen to avoid adding complexity and dependencies.

**Impact:** Callers receive numbers as `"42"` strings, booleans as `"t"`/`"f"`. This is sufficient for Phase 5 unblocking (pgvector schema extensions + SDK generation). Before Phase 5 ships a production vector search surface, the row serializer should be upgraded to typed JSON deserialization.

**Corrective action:** Add a task to `openspec/changes/p2-c006-pgvector-rpc/tasks.md` or create a follow-on change for typed column deserialization in `PgVectorRpc::execute_similarity()`.

---

## Technical Debt Introduced

| Item | Location | Severity | When to address |
|------|----------|----------|-----------------|
| Text-format row serialization in PgVectorRpc | `crates/fdb-postgres/src/lib.rs:271-282` | MEDIUM | Before Phase 5 pgvector surface ships to production SDKs |
| pgvector crate version annotation in proposal | `openspec/changes/p2-c006-pgvector-rpc/proposal.md` | LOW | Before Phase 5 assessment reads it |
| PgVectorRpc bypasses port trait (no `VectorSimilarity` trait) | `crates/fdb-gateway/src/main.rs` | LOW | When a second vector backend is needed; MVP deferred intentionally |
| `filter` field in VectorRpcRequest is accepted but not applied | `crates/fdb-postgres/src/lib.rs:236` | LOW | Phase 5 — attach as additional WHERE clause |

---

## Lessons

1. **Rust crate version ≠ Postgres extension version.** The `pgvector` Rust crate (0.4.x series) and the `pgvector` Postgres extension (0.7.x series) have independent versioning. Spec documents should specify both axes separately.

2. **todo!() startup panics block everything downstream.** The `OpenApiCompiler::compile()` panic prevented end-to-end gateway testing. Ordering p2-c007 first (plan decision) was correct and necessary — this ordering rule should be enforced in future phases: any change replacing a `todo!()` in a startup path takes highest priority.

3. **SQL injection guard placement.** Validating identifiers at the adapter boundary (before `format!` macro, not at the handler) is the correct layer. The `is_safe_identifier()` function in `fdb-postgres` enforces this — not the handler in `fdb-gateway`. Domain input validation at system boundaries, not at composition roots.

4. **Clippy pedantic catches real issues.** The redundant closure `|v| serde_json::Value::String(v)` was flagged correctly. Running `cargo clippy -D warnings` as a mandatory gate before marking changes complete is effective.

---

## Phase Outcome

**COMPLETE — 2/2 goals MET.** The gateway no longer panics on startup. Vector similarity search is available at `POST /rpc/vector` under full RLS context. `GET /openapi.json` serves a hot-reloaded OpenAPI 3.1.0 document. Phase 5 `p5-c001` is unblocked.

---

## Recommended Next Phase

**p3-graphql-hybrid-engine**

The composition root (`fdb-gateway`) now boots cleanly with OpenAPI served and a functional vector route. The next critical surface is the GraphQL hybrid engine (p3), which provides the query/mutation delegation to `graphql.resolve()` and subscription fan-out via `async-graphql`. This phase completes the core Quarry API surface.

**First change when resuming p3:** `p3-c008-extended-guc-propagation` (the 6-GUC SET LOCAL block was extended in a prior session but needs verification against the full test suite with real subscription paths), then `p3-c001-graphql-passthrough`.

**Pre-kickoff gate for p3:** OQ-3 (pg_graphql PG18 tagged release) must be confirmed resolved (`docs/contracts/pg-graphql-version.md`) before coding `p3-c001`.
