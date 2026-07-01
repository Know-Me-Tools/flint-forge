# Plan — p2-quarry-backfill

**Date:** 2026-06-30
**Phase:** p2-quarry-backfill
**Backend:** OpenSpec (`openspec/changes/` — both proposals already exist)
**Changes:** 2
**Execution order:** p2-c007 → p2-c006

---

## Pre-Execution Summary

Both OpenSpec proposals and tasks files exist and are fully specified:
- `openspec/changes/p2-c007-openapi-compiler/` — proposal.md + tasks.md (T1–T10)
- `openspec/changes/p2-c006-pgvector-rpc/` — proposal.md + tasks.md (T0–T6)

**OQ-9 CLEARED** (assessment): `postgresql-18-pgvector` already installed in
`images/postgres18/Dockerfile`. T0 from p2-c006 tasks is resolved — no Dockerfile
change needed. pgvector ≥ 0.7.0 in PGDG.

**Critical path blocker:** `OpenApiCompiler::compile()` is a `todo!()` that panics
at startup. p2-c007 executes first to unblock end-to-end gateway boot.

---

## Change 1 (of 2): p2-c007-openapi-compiler

**Priority:** P1  
**Spec:** `openspec/changes/p2-c007-openapi-compiler/proposal.md`  
**Tasks:** `openspec/changes/p2-c007-openapi-compiler/tasks.md`  
**Agent:** built-in (direct implementation)

### Why first
`OpenApiCompiler::compile()` is a `todo!()` macro. `StateManager::do_compile()` calls
it synchronously at startup (`new().await`) and on every DDL change. Any hot-reload
or first-boot attempt panics. p2-c007 must land before the gateway is exercisable
end-to-end.

### Scope (reconciled with assessment)

Two sub-deliverables:

**Sub-A — `OpenApiCompiler::compile()` in `fdb-reflection`**  
File: `crates/fdb-reflection/src/compilers/openapi.rs`  
Tasks: T1 (type mapping), T2 (table schema), T3 (filter params), T4 (table paths),
T5 (fn path), T6 (assemble document)  
Dependencies: `serde_json` already in `fdb-reflection/Cargo.toml`. No new deps.

**Sub-B — `GET /openapi.json` route in `fdb-gateway`**  
File: `crates/fdb-gateway/src/main.rs`  
Task: T7  
Handler reads `state_manager.current().openapi_doc` and returns `Json(...)`.  
No auth — OpenAPI docs are public (same pattern as Supabase's `/rest/v1/` endpoint).

### Constraint notes
- No new crate deps. BLOCK constraint (new dep without workspace check) does not apply.
- `OpenApiCompiler` lives in `fdb-reflection` (adapter layer). Does NOT import `fdb-gateway`. Hexagonal rule satisfied.
- `compile()` signature: `pub fn compile(model: &DatabaseModel) -> serde_json::Value`. This is unchanged from the stub.
- File size: `openapi.rs` will be ~200–280 lines (within 500-line BLOCK limit).

### Tests (tasks T8, T9)
Location: inline in `crates/fdb-reflection/src/compilers/openapi.rs` as `#[cfg(test)]` block:
- `test_openapi_version_is_3_1_0`
- `test_every_table_has_crud_paths`
- `test_column_types_map_correctly`
- `test_function_has_post_path`
- `test_bearer_security_scheme_present`
- `test_filter_params_for_column_present`

Integration test (T9) deferred — no live DB in unit test context; the gateway route
is verified by checking the JSON is a valid Value (non-null).

### Gate
`cargo test -p fdb-reflection -- openapi` passes.
`cargo check --workspace` passes.
`cargo clippy --workspace -- -D warnings` clean.

---

## Change 2 (of 2): p2-c006-pgvector-rpc

**Priority:** P1  
**Spec:** `openspec/changes/p2-c006-pgvector-rpc/proposal.md`  
**Tasks:** `openspec/changes/p2-c006-pgvector-rpc/tasks.md`  
**Agent:** built-in (direct implementation)

### Why second
Depends on `fdb-domain` stability. Can execute after p2-c007 since both are
independent — but p2-c007 is the startup-crash fix and should be confirmed
green first.

### Scope (reconciled with assessment — MVP adjustment)

The original proposal scopes p2-c006 as a *general-purpose RPC pgvector adapter*
(detect vector args in any reflected function, T2–T4 in the original tasks).
The assessment identified a simpler MVP path: a **dedicated `/rpc/vector` endpoint**
for vector similarity search (not general-purpose pgvector arg injection into every
RPC handler).

**Plan decision: implement the dedicated-endpoint MVP.**

Rationale:
- The general-purpose adapter (T2–T4) requires `ReflectionEngine` changes and a
  live DB to test properly — that's integration-test territory gated by a running
  PG18 container with real functions.
- The dedicated `/rpc/vector` endpoint is self-contained, unit-testable, and
  directly unblocks Phase 5 `p5-c001` (pgvector schema).
- General-purpose pgvector arg injection can land in a follow-on change against
  `p2-c004` (REST compiler query builder) when that lands.

### Files affected (MVP scope)

| File | Action |
|------|--------|
| `Cargo.toml` (workspace `[dependencies]`) | Add `pgvector = { version = "0.7", features = ["postgres"] }` |
| `crates/fdb-domain/src/lib.rs` | Add `VectorRpcRequest` struct |
| `crates/fdb-postgres/Cargo.toml` | Add `pgvector = { workspace = true }` |
| `crates/fdb-postgres/src/lib.rs` | Add `PgVectorRpc` struct + `execute_similarity()` |
| `crates/fdb-gateway/src/main.rs` | Add `POST /rpc/vector` route + `VectorRpcBody` handler |

### VectorRpcRequest (fdb-domain, Layer 0)

```rust
#[derive(Debug, Clone, serde::Deserialize)]
pub struct VectorRpcRequest {
    pub embedding: Vec<f32>,
    pub table: String,
    pub column: String,
    #[serde(default = "default_limit")]
    pub limit: u32,
    pub filter: Option<serde_json::Value>,
}
fn default_limit() -> u32 { 10 }
```

No infra deps — zero-cost addition to `fdb-domain`.

### PgVectorRpc (fdb-postgres, adapter layer)

SQL executed under full 6-GUC RLS context via `PgBackend::acquire()`:

```sql
SELECT *, ({column} <=> $1::vector) AS distance
FROM {schema}.{table}
ORDER BY {column} <=> $1::vector
LIMIT $2
```

Table and column names are validated against an allowlist pattern
(`^[a-zA-Z_][a-zA-Z0-9_.]*$`) before interpolation — no user-controlled
SQL injection. Limit is capped at 1000.

The `pgvector` crate's `Vector` type (with `features = ["postgres"]`) provides
`ToSql` impl for `tokio-postgres`.

### Gateway handler (fdb-gateway)

```rust
// POST /rpc/vector
// No auth-free access — bearer required, same as /graphql
async fn rpc_vector_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(body): Json<VectorRpcBody>,
) -> impl IntoResponse
```

Uses the same `extract_bearer()` + `rls_from_bearer()` pattern already in the
gateway. Returns JSON rows or appropriate error.

**WARN**: `VectorRpcBody` references `VectorRpcRequest` from `fdb-domain` — no new
domain type constraint violations.

### Constraint notes
- `pgvector = "0.7"` is a new workspace dep — checked: not currently in workspace
  `[dependencies]`. WARN constraint triggers. Justification: pgvector is the only
  Rust crate that provides typed `ToSql`/`FromSql` for the `vector` Postgres type.
  No alternative in the workspace satisfies this.
- Table/column name interpolation: validated with regex before SQL construction.
  Prevents injection. BLOCK for SQL injection does not apply when properly guarded.
- `PgVectorRpc` lives in `fdb-postgres` (adapter). The gateway calls it directly
  (same pattern as `PgGraphQl`). No port trait addition needed for MVP.

### Tests

Location: `#[cfg(test)]` block in `crates/fdb-postgres/src/lib.rs` or new
`crates/fdb-postgres/src/vector_rpc.rs`:

- `test_vector_rpc_request_deserializes_correctly`
- `test_table_name_validation_rejects_injection_attempts`
- `test_limit_capped_at_1000`
- `#[ignore]` integration marker: `test_similarity_query_executes_under_rls` (requires `DATABASE_URL`)

### Gate

`cargo test -p fdb-postgres` passes.
`cargo test -p fdb-gateway` passes (route registration).
`cargo check --workspace` passes.
`cargo clippy --workspace -- -D warnings` clean.

---

## Execution Order

```
1. p2-c007-openapi-compiler
   ├─ Sub-A: implement OpenApiCompiler::compile() in fdb-reflection
   └─ Sub-B: add GET /openapi.json to fdb-gateway
   → gate: cargo test -p fdb-reflection -- openapi; cargo check --workspace PASS

2. p2-c006-pgvector-rpc
   ├─ T1: add pgvector to workspace Cargo.toml + fdb-postgres/Cargo.toml
   ├─ T2: add VectorRpcRequest to fdb-domain
   ├─ T3: implement PgVectorRpc in fdb-postgres
   └─ T4: add POST /rpc/vector to fdb-gateway
   → gate: cargo test -p fdb-postgres; cargo check --workspace PASS
```

---

## Phase Complete When

- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean
- [ ] `GET /openapi.json` route registered and returns non-panicking JSON (tested)
- [ ] `OpenApiCompiler::compile(&minimal_model())` produces `openapi: "3.1.0"` (unit tested)
- [ ] `POST /rpc/vector` route registered (unit tested via route-registration test)
- [ ] `VectorRpcRequest` validation unit tested (injection guard confirmed)
- [ ] Both changes marked `qa_passed` in `progress.json`

---

## Out of Scope (this phase)

- General-purpose pgvector arg injection into every reflected function RPC handler (p2-c006 original T2–T4 full scope) — deferred to p2-c004 follow-on
- OpenAPI HTTP integration test requiring a live gateway (deferred — no test DB in unit context)
- `utoipa` structured builder (hand-rolled serde_json::Value per proposal decision)
- pgvector HNSW index management (Phase 5 operations concern)
