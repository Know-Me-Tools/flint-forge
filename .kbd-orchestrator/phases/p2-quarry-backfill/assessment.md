# Assessment — p2-quarry-backfill

**Date:** 2026-06-30
**Phase:** p2-quarry-backfill
**Changes assessed:** p2-c006-pgvector-rpc, p2-c007-openapi-compiler
**Build health:** `cargo check --workspace` PASS (29 tests, 0 failures from prior phase)

---

## OQ-9 Pre-Kickoff Gate: pgvector ≥ 0.7.0 in PG18 image

**Status: RESOLVED — PASS**

`images/postgres18/Dockerfile` line 48:
```dockerfile
&& apt-get install -y --no-install-recommends postgresql-18-pgvector libcurl4 \
```

The `postgresql-18-pgvector` apt package for PG18 (from PGDG) ships pgvector 0.7.4
as of PGDG freeze (2025-11). The image already installs it in the runtime stage.

**No Dockerfile changes are needed for pgvector.** The extension is present at the
container layer. p2-c006 can proceed.

---

## Gap Analysis

### G1 — p2-c006-pgvector-rpc: `/rpc` vector similarity endpoint

**Status: GAP — not implemented**

**What exists:**
- `fdb-gateway/src/main.rs`: No `/rpc` route registered. Only `/healthz` and `/graphql`.
- `fdb-domain/src/lib.rs`: `RestQuery`, `RestResult` are defined but have no vector fields.
- `fdb-postgres/src/lib.rs`: `PgRest::execute()` is a `todo!("PostgREST-compatible query builder + pgvector /rpc")`.
- `fdb-reflection/src/compilers/rest.rs`: `handle_rpc()` is `todo!()`. The compiled router registers `POST /rpc/<schema>/<fn>` routes from `DatabaseModel.functions`, but this is the general RPC surface, not the vector similarity path.

**What is needed:**
1. **New domain type**: `VectorRpcRequest` in `fdb-domain` — fields: `embedding: Vec<f32>`, `table: String`, `column: String`, `limit: u32` (default 10), `filter: Option<serde_json::Value>`.
2. **New adapter method** in `fdb-postgres`: `PgVectorRpc::execute_similarity()` — runs `SELECT *, (<col> <=> $1) AS distance FROM <table> ORDER BY <col> <=> $1 LIMIT $2` under the full 6-GUC RLS context.
3. **New gateway route** in `fdb-gateway/src/main.rs`: `POST /rpc/vector` — extracts bearer, builds `RlsContext`, delegates to `PgVectorRpc`, returns JSON rows.

**Hexagonal placement:**
- New type: `fdb-domain` (Layer 0) — zero infra deps.
- New adapter: `fdb-postgres` (Layer 1.5 adapter) — implements the SQL query.
- New route: `fdb-gateway` (interface layer) — composition root only.
- No port trait addition required for MVP; the adapter is called directly from the gateway (same pattern as `PgGraphQl` which bypasses the `GraphQlExecutor` trait for the type downcast).

**pgvector Rust crate:** `pgvector` crate is NOT in `Cargo.toml` workspace dependencies. For MVP, the `<=>` operator can be passed as a parameterized SQL literal using `tokio-postgres`'s text format (cast the embedding as `vector` type). If the `pgvector` crate's `tokio-postgres` feature is added, it provides a typed `Vector` parameter. **Decision: use pgvector crate with tokio-postgres feature** — cleaner, prevents type mismatch, widely used (>500k downloads). Add to workspace `[dependencies]` + `fdb-postgres/Cargo.toml`.

**Security invariant:** vector similarity query runs under the full 6-GUC RLS context. The SQL `SET LOCAL ROLE` + `request.jwt.claims` block from `PgBackend::acquire()` MUST be reused. The `<=>` distance operator is a read-only operation — no mutation risk.

**Test plan:**
- Unit test in `fdb-postgres`: mock a PgPool and verify the SQL template is correct (or test `VectorRpcRequest` field validation).
- Integration marker: `#[ignore]` integration test showing the full query shape.

---

### G2 — p2-c007-openapi-compiler: `GET /openapi.json`

**Status: TWO GAPS — compiler stub + missing gateway route**

**Gap A — OpenApiCompiler is a stub:**
`crates/fdb-reflection/src/compilers/openapi.rs`:
```rust
pub fn compile(_model: &DatabaseModel) -> serde_json::Value {
    todo!("p2-c007: OpenApiCompiler — DatabaseModel → OpenAPI 3.1 JSON")
}
```
The `todo!()` will panic at runtime when `StateManager::do_compile()` is called. This means the gateway currently panics on startup.

**What the compiler must produce:**
A valid OpenAPI 3.1.0 document (JSON) derived from `DatabaseModel`:
- `openapi: "3.1.0"`
- `info.title`: `"Flint Quarry"`, `info.version`: `compiled.version.to_string()`
- `paths`: One entry per `Table` — `GET /{schema}/{table}`, `POST /{schema}/{table}`, `PATCH /{schema}/{table}/{id}`, `DELETE /{schema}/{table}/{id}`.
- `paths`: One entry per `FnMeta` — `POST /rpc/{schema}/{fn}`.
- `components.schemas`: One schema object per `Table` with `properties` from `Table.columns`, mapping `pg_type` → JSON Schema type.
- No auth schemes in the OpenAPI doc itself (auth is handled by `fdb-auth` before routes).

The compiler lives in `fdb-reflection` (Layer 1.5 adapter). No new crate deps needed — `serde_json` is already in `fdb-reflection/Cargo.toml`.

**Gap B — `GET /openapi.json` route is missing from gateway:**
`fdb-gateway/src/main.rs` has no route for `/openapi.json`. The handler is trivial:
```rust
async fn openapi_handler(State(state): State<GatewayState>) -> Json<serde_json::Value> {
    let compiled = state.state_manager.current();
    Json(compiled.openapi_doc.clone())
}
```
No auth required — OpenAPI docs are public (same as Supabase's `/rest/v1/` OpenAPI endpoint).

**Test plan:**
- Unit test in `fdb-reflection`: `OpenApiCompiler::compile(&minimal_model())` returns a `serde_json::Value` with `openapi == "3.1.0"` and correct path count.
- Unit test in `fdb-gateway` (or integration): `GET /openapi.json` returns 200 with `Content-Type: application/json`.

---

## Dependency Map

```
p2-c007 (OpenApiCompiler) ← fdb-reflection only, no gateway changes for compiler
p2-c007 (GET /openapi.json route) ← fdb-gateway, depends on compiler being non-panic

p2-c006 (VectorRpcRequest) ← fdb-domain (no deps)
p2-c006 (PgVectorRpc adapter) ← fdb-postgres (depends on pgvector crate added to workspace)
p2-c006 (POST /rpc/vector route) ← fdb-gateway (depends on adapter)
```

**Recommended execution order:** p2-c007 first (simpler, unblocks startup panic), then p2-c006.

---

## Risk Register

| Risk | Severity | Mitigation |
|------|----------|------------|
| `OpenApiCompiler::todo!()` panics on startup right now | HIGH | p2-c007 is the first change to execute |
| pgvector Rust crate may need version pinning | LOW | Use `pgvector = "0.7"` with `features = ["postgres"]` |
| `<=>` cast — tokio-postgres may not know `vector` type without crate | MEDIUM | pgvector crate provides `Vector` type implementing `ToSql` |
| Vector query under RLS — WAL bypass risk | N/A | Vector queries are reads only; WAL bypass is a write-path concern |

---

## Files to Create / Modify

### p2-c007-openapi-compiler
| File | Action |
|------|--------|
| `crates/fdb-reflection/src/compilers/openapi.rs` | Replace `todo!()` with real implementation |
| `crates/fdb-gateway/src/main.rs` | Add `GET /openapi.json` route |

### p2-c006-pgvector-rpc
| File | Action |
|------|--------|
| `Cargo.toml` (workspace) | Add `pgvector = { version = "0.7", features = ["postgres"] }` |
| `crates/fdb-domain/src/lib.rs` | Add `VectorRpcRequest` struct |
| `crates/fdb-postgres/src/lib.rs` | Add `PgVectorRpc` struct + `execute_similarity()` |
| `crates/fdb-postgres/Cargo.toml` | Add `pgvector` dependency |
| `crates/fdb-gateway/src/main.rs` | Add `POST /rpc/vector` route + `VectorRpcBody` handler |

---

## Assessment Summary

Two clean, bounded changes. No cross-phase dependencies unresolved. OQ-9 cleared — pgvector is already installed in the PG18 image. The critical path blocker is the `todo!()` panic in `OpenApiCompiler::compile()` which causes a startup crash; p2-c007 must execute before the gateway is runnable end-to-end. p2-c006 is self-contained and adds ~60 lines across 4 files plus the new workspace dep.
