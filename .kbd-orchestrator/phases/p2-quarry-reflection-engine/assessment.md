# Assessment — p2-quarry-reflection-engine

**Date:** 2026-06-30  
**Assessor:** kbd-assess  
**Phase gate:** RLS-correct REST CRUD under a real flint-gate JWT; ArcSwap hot-swap end-to-end within 5s; zero dropped requests during schema reload.

---

## Phase 1 Foundation Verified ✓

All Phase 1 prerequisites are confirmed present in the codebase:

| Prerequisite | Status | Evidence |
|---|---|---|
| `flint_meta` schema + DDL triggers | DELIVERED | `crates/ext-flint-meta/src/` — schema.rs, triggers.rs, functions.rs, agui.rs |
| `forge-identity::RlsContext` struct | DELIVERED | `crates/forge-identity/src/lib.rs:18–36` — `RlsContext { role, claims_json, raw_bearer }` |
| `forge-identity::verify_and_build()` stub | DELIVERED (stub) | `lib.rs:50` — `todo!()` body, correct signature |
| JWT contract pinned | DELIVERED | `docs/contracts/jwt-contract.md` — CRITICAL role-claim note at lines 153–157 |
| `fdb-domain` types | DELIVERED | `TableMeta`, `ColumnMeta`, `RestQuery`, `RestResult`, `GraphQlRequest`, `ChangeEvent`, `SchemaVersion` |
| `fdb-ports` trait seams | DELIVERED | `DatabaseBackend`, `SchemaProvider`, `RestExecutor`, `GraphQlExecutor`, `ChangeStreamSource` |
| Workspace Cargo.toml | DELIVERED | `arc-swap = "1"` already in workspace deps; `axum = "0.8.8"`, `sqlx` NOT yet in workspace |
| `cargo check --workspace` passes | VERIFIED | Clean build (0.41s, no warnings) |

---

## Gap Analysis

### G1 — p2-c001: `fdb-auth` — JWT verify → `RlsContext`

**Current state:** Shell. `crates/fdb-auth/src/lib.rs` has the correct function signature calling `forge_identity::verify_and_build()`, but that function is a `todo!()`. The Cargo.toml comment notes `jsonwebtoken + JWKS cache` are missing.

**Gap:** `forge-identity::verify_and_build()` does not actually verify JWTs. The full JWKS fetch → signature validation → claims decode → `RlsContext` assembly pipeline is unimplemented.

**Specific work required:**
1. Add `jsonwebtoken`, `reqwest` (for JWKS), and a `once_cell`/`tokio::sync::OnceCell` JWKS cache to `fdb-auth/Cargo.toml`
2. Implement `verify_and_build()` in `forge-identity` (or delegate to `fdb-auth`)
3. Read issuer and JWKS URL from environment (`FLINT_GATE_ISSUER`, `FLINT_GATE_JWKS_URL`)
4. Apply the `role` claim critical rule: if no `role` in claims, coerce to `"anon"` (not error)
5. Populate `RlsContext { role, claims_json, raw_bearer }` from verified token

**Blocker for:** p2-c002 (pool uses `RlsContext`), all REST requests.

---

### G2 — p2-c002: `fdb-postgres` — deadpool pool + `SET LOCAL` RLS context

**Current state:** Shell. `PgBackend`, `PgGraphQl`, `PgRest` structs exist with `todo!()` bodies. `Cargo.toml` notes deadpool-postgres missing.

**Gap:** No actual Postgres connection pool. The three `SET LOCAL` statements are not implemented. The `/* pool */` field on `PgBackend` is a comment placeholder.

**Specific work required:**
1. Add `deadpool-postgres`, `tokio-postgres`, `sqlx` (for PgListener in p2-c005) to workspace + `fdb-postgres/Cargo.toml`
2. Implement `PgBackend::acquire()`: checkout from pool, run:
   ```sql
   SET LOCAL ROLE <rls.role>;
   SET LOCAL "request.jwt.claims" = '<rls.claims_json>';
   SET LOCAL "request.headers"    = '{"authorization": "Bearer <rls.raw_bearer>"}';
   ```
3. These must be inside the same transaction. The `Conn` wrapper must hold the tokio-postgres `Transaction` (not a `Client`) so the `SET LOCAL` statements don't escape.
4. Note: `SET LOCAL` requires an open transaction — the connection must begin a transaction before setting locals. This aligns with the per-request transaction model.

**Security contract:** `raw_bearer` is never logged. `claims_json` is never logged. These propagate to `request.headers` and `request.jwt.claims` GUC parameters only.

---

### G3 — p2-c003: `fdb-reflection` crate — NEW CRATE, does not exist

**Current state:** `fdb-reflection` crate does NOT exist. No directory, no `Cargo.toml`, no source files.

**Gap:** This is the largest single deliverable. The entire `DatabaseModel` IR, `CompiledState`, `StateManager`, and `ReflectionEngine` must be created from scratch.

**Specific work required — new crate `crates/fdb-reflection/`:**
```
fdb-reflection/
├── Cargo.toml           (adapter crate: imports fdb-domain, fdb-ports; NOT fdb-gateway)
└── src/
    ├── lib.rs           (pub re-exports: DatabaseModel, CompiledState, StateManager, ReflectionEngine)
    ├── model.rs         (DatabaseModel, Table, Column, Relationship, Function, ViewMeta)
    ├── compiled.rs      (CompiledState { version, database_model, router, openapi_doc, mcp_tools, agui_descriptors })
    ├── state_manager.rs (StateManager { compiled: ArcSwap<CompiledState>, db_pool, config })
    ├── engine.rs        (ReflectionEngine::reflect() — sqlx queries against flint_meta.*)
    ├── error.rs         (ReflectionError: thiserror, #[non_exhaustive])
    ├── passes/
    │   ├── mod.rs
    │   ├── normalization.rs
    │   ├── validation.rs
    │   ├── permission_analysis.rs
    │   └── endpoint_generation.rs
    └── compilers/
        ├── mod.rs
        ├── rest.rs      (Phase 2: DatabaseModel → axum::Router<()>)
        ├── openapi.rs   (Phase 2: DatabaseModel → utoipa::OpenApi)
        ├── graphql.rs   (Phase 3 stub — file exists but body is todo!())
        └── mcp.rs       (Phase 7 stub — file exists but body is todo!())
```

**Hexagonal rule enforcement:** `fdb-reflection` MUST NOT import `fdb-gateway`. `fdb-gateway` imports `fdb-reflection`. This is the adapter→interface layering rule.

**Workspace registration:** Add `"crates/fdb-reflection"` to `[workspace] members` in root `Cargo.toml`.

**Key types to define in `model.rs`:**
- `DatabaseModel { tables: Vec<Table>, functions: Vec<FnMeta>, views: Vec<ViewMeta>, version: u64 }`
- `Table { schema, name, columns, pk, fk, rls_enabled, vault_key: Option<EncryptedDek> }`
- `EncryptedDek(Vec<u8>)` — ciphertext ONLY; no plaintext key material in `DatabaseModel`

**`ReflectionEngine::reflect()` must call:**
- `SELECT * FROM flint_meta.tables()`
- `SELECT * FROM flint_meta.columns($schema, $table)` for each table
- `SELECT * FROM flint_meta.relationships()`
- `SELECT * FROM flint_meta.functions()`
- `SELECT * FROM flint_meta.version()` → sets `DatabaseModel.version`

---

### G4 — p2-c004: `RestCompiler` — part of `fdb-reflection`

**Current state:** Does not exist (blocked by G3).

**Gap:** `fdb-postgres/src/lib.rs:35` has `PgRest::execute()` as `todo!()` and currently delegates to the old PostgREST-compat path. Phase 2 replaces this with the `RestCompiler` in `fdb-reflection`.

**Specific work required:**
1. Implement `compilers/rest.rs` — `RestCompiler::compile(model: &DatabaseModel) -> axum::Router<()>`
2. Route pattern: one route per table for CRUD + `/rpc/:fn_name` for functions
3. Parameterized SQL only — no string interpolation of user values. Column names validated against `DatabaseModel` before use in ORDER BY / SELECT.
4. Filter operators: `eq`, `neq`, `gt`, `gte`, `lt`, `lte`, `like`, `ilike`, `in`, `is`, `cs` (contains), `cd` (contained by)
5. Range header pagination: `Range: items=0-24` → `LIMIT 25 OFFSET 0`
6. HTTP method → SQL verb:
   - `GET` → parameterized `SELECT`
   - `POST` → `INSERT … RETURNING *`
   - `PATCH` → `UPDATE … WHERE id = $1 RETURNING *`
   - `DELETE` → `DELETE … WHERE id = $1 RETURNING *`
7. Each handler acquires a `Conn` from `PgBackend::acquire(rls)` — the `SET LOCAL` block runs inside the transaction before the user query.

---

### G5 — p2-c005: `StateManager::start_listener()` — PgListener hot-swap

**Current state:** Does not exist (blocked by G3).

**Gap:** No hot-reload loop. `sqlx` is not in workspace deps.

**Specific work required:**
1. Add `sqlx = { version = "0.8", features = ["postgres", "runtime-tokio", "uuid", "json"] }` to workspace Cargo.toml
2. Implement `state_manager.rs` per the spec in `docs/FLINT-PHASE-PLAN-REVISED.md §Phase 2`
3. Critical: reconnect loop is NOT built into `sqlx::postgres::PgListener` — must implement retry with `tokio::time::sleep` in the outer `loop`
4. On reconnect: force a full `ReflectionEngine::reflect()` recompile — notifications may have been missed
5. Serve old `ArcSwap` state during compilation — no request downtime

---

### G6 — p2-c007: `OpenApiCompiler`

**Current state:** Does not exist (blocked by G3).

**Gap:** `GET /openapi.json` route in `fdb-gateway` returns nothing.

**Specific work required:**
1. Add `utoipa = "4"` (or compatible version) to workspace Cargo.toml
2. Implement `compilers/openapi.rs` — `OpenApiCompiler::compile(model: &DatabaseModel) -> utoipa::openapi::OpenApi`
3. Each table → OpenAPI path with GET/POST/PATCH/DELETE operations + schema from `ColumnMeta`
4. Mount at `GET /openapi.json` in `fdb-gateway`
5. Spec note: this is P1 — can ship after the REST MVP gate passes

---

### G7 — Gate Tests

**Current state:** Only `crates/fdb-app/tests/meta_listener.rs` exists (Phase 1 gate test for `sqlx::PgListener` on `meta_runtime`). No Phase 2 REST tests yet.

**Gap:** All REST compiler gate tests are absent.

**Required tests (in `crates/fdb-reflection/tests/`):**
- `test_rest_select_with_eq_filter` — parameterized SELECT with `eq` operator
- `test_rest_select_all_filter_operators` — covers all 12 operators
- `test_rest_insert_returns_row` — POST → INSERT RETURNING
- `test_rest_patch_by_id` — PATCH → UPDATE by PK
- `test_rest_delete_by_id` — DELETE by PK
- `test_hot_swap_no_dropped_requests` — concurrent load during ArcSwap store
- `test_vault_dek_not_in_compiled_state` — assert no `plaintext_dek` field anywhere in `CompiledState` or `DatabaseModel` (security gate)

---

## Dependency Gaps (workspace Cargo.toml)

The following dependencies are needed in `[workspace.dependencies]` but not yet present:

| Dependency | Version | Required by |
|---|---|---|
| `sqlx` | `"0.8"` with `postgres, runtime-tokio, uuid, json` features | p2-c003 (PgListener), p2-c005 |
| `deadpool-postgres` | `"0.14"` | p2-c002 |
| `tokio-postgres` | `"0.7"` | p2-c002 (via deadpool-postgres) |
| `jsonwebtoken` | `"9"` | p2-c001 (JWT verify) |
| `utoipa` | `"4"` | p2-c007 (OpenAPI) |
| `reqwest` | `"0.12"` with `json` feature | p2-c001 (JWKS fetch) |

Already present in workspace: `tokio`, `axum = "0.8.8"`, `arc-swap = "1"`, `serde`, `serde_json`, `thiserror`, `tracing`, `futures`.

---

## Current `fdb-gateway` Gap

`fdb-gateway/src/main.rs` only has the `/healthz` route. It needs to:
1. Initialize `deadpool-postgres` pool from env
2. Run `ReflectionEngine::reflect()` at startup
3. Start `StateManager::start_listener()` background task
4. Mount the compiled `Router<()>` from `ArcSwap` via a thin dispatch handler
5. Mount `GET /openapi.json` (P1, after MVP)

The gateway currently has NO dependency on `fdb-reflection` — that import will need adding.

---

## Risk Register (inherited + new)

| Risk | Likelihood | Phase 2 Impact | Mitigation |
|---|---|---|---|
| sqlx version conflict with deadpool-postgres | LOW | Build break | Pin compatible versions: deadpool-postgres 0.14 uses tokio-postgres 0.7; sqlx 0.8 is independent |
| `PgListener` loses connection under load; recompile missed | MEDIUM | Stale schema served | Reconnect loop with forced recompile on reconnect — already spec'd |
| REST compiler generates unparameterized SQL for column names in ORDER BY | HIGH (security) | SQL injection vector | Validate column names against `DatabaseModel.tables[].columns` before use in SQL; return `400` if unknown |
| `role` claim absent from JWT → all RLS policies see `anon` | HIGH (silent) | RLS bypass (permissive direction) | Coerce missing `role` to `"anon"` in `RlsContext`; document this in code comment; NOT an error |
| `CompiledState` accidentally stores plaintext DEK | HIGH (security) | Credential exposure | Only store `EncryptedDek(Vec<u8>)` in `DatabaseModel`; no `plaintext` field anywhere. Gate test verifies. |
| `SET LOCAL` not in transaction | MEDIUM | GUC leaks to next request | `Conn` wrapper must begin and own a `tokio_postgres::Transaction` — `SET LOCAL` only persists within transaction |
| `fdb-reflection` crate imports `fdb-gateway` accidentally | LOW | Circular dependency | CI gate: `cargo check --workspace` will catch the cycle |

---

## Summary of Gaps (ordered by dependency)

1. **[MISSING CRATE]** `fdb-reflection` — entire new crate; largest deliverable
2. **[STUB]** `forge-identity::verify_and_build()` — JWT JWKS verify unimplemented
3. **[STUB]** `PgBackend::acquire()` — no actual pool or `SET LOCAL` RLS propagation
4. **[MISSING DEPS]** `sqlx`, `deadpool-postgres`, `jsonwebtoken`, `utoipa`, `reqwest` not in workspace
5. **[GATEWAY]** `fdb-gateway` not wired to reflection engine or compiled router
6. **[TESTS]** All Phase 2 gate tests absent

**Nothing from Phase 2 has been implemented yet.** The workspace compiles cleanly because all bodies are `todo!()` stubs. This is the expected scaffold state for Phase 1 completion.

---

## Assessment Outcome

**Status: ASSESSMENT_COMPLETE — Ready to plan**  
**Gaps found: 6 major**  
**Prerequisite integrity: PASS (Phase 1 fully delivered)**  
**Build health: GREEN (cargo check passes)**

The assessment confirms Phase 2 starts from a correct but stub-only foundation. The spec in `docs/FLINT-PHASE-PLAN-REVISED.md §Phase 2` is complete and actionable. The primary planning decision is whether `fdb-auth` (p2-c001) and `fdb-postgres` (p2-c002) should be parallelized with `fdb-reflection` (p2-c003) scaffold, or sequenced strictly.

**Recommended plan order:** p2-c003 scaffold first (defines the IR types all other work depends on) → p2-c001 + p2-c002 in parallel → p2-c004 (REST compiler, needs IR) → p2-c005 (hot-reload, needs compiler) → p2-c007 (OpenAPI, P1 after gate).
