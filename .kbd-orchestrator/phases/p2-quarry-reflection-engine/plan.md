# Plan — p2-quarry-reflection-engine

**Date:** 2026-06-30  
**Planner:** kbd-plan  
**Change Backend:** OpenSpec (`openspec/changes/p2-*/`)

---

## Phase Gate (Reminder)

Success = RLS-correct REST CRUD under a real flint-gate JWT; ArcSwap hot-swap
end-to-end within 5 seconds of DDL change; zero dropped requests during reload.

---

## Ordered Change List

### MVP Block (P0 — must complete in order for gate to pass)

| Order | Change ID | Crate | Summary | Agent |
|---|---|---|---|---|
| 1 | `p2-c003-flint-reflection-crate` | `fdb-reflection` (NEW) | Create the entire IR crate: DatabaseModel, CompiledState, ReflectionEngine, pipeline passes, compiler stubs | rust-reviewer |
| 2a (parallel) | `p2-c001-fdb-auth` | `forge-identity` / `fdb-auth` | JWT JWKS verify → RlsContext | security-reviewer |
| 2b (parallel) | `p2-c002-fdb-postgres` | `fdb-postgres` | deadpool-postgres pool + SET LOCAL RLS propagation | rust-reviewer |
| 3 | `p2-c004-rest-compiler` | `fdb-reflection/compilers/rest.rs` | RestCompiler + CRUD handlers | rust-reviewer |
| 4 | `p2-c005-arcswap-hot-reload` | `fdb-reflection/state_manager.rs` | StateManager PgListener loop + fdb-gateway wiring | rust-reviewer |

### Post-MVP (P1 — ship after gate passes)

| Order | Change ID | Crate | Summary | Agent |
|---|---|---|---|---|
| 5 | `p2-c007-openapi-compiler` | `fdb-reflection/compilers/openapi.rs` | OpenApiCompiler + GET /openapi.json | rust-reviewer |
| 6 | `p2-c006-pgvector-rpc` | `fdb-reflection/compilers/handlers.rs` | pgvector vector type support in RPC handler | rust-reviewer |

---

## Ordering Rationale

**p2-c003 first:** `fdb-reflection` defines the IR types (`DatabaseModel`,
`CompiledState`, `Table`, `Column`, etc.) that every other change depends on.
Even though `p2-c001` and `p2-c002` modify different crates, they both
ultimately need to interoperate with `fdb-reflection` for the handlers to work.
The crate must exist and compile before work on compilers or StateManager begins.

**p2-c001 + p2-c002 in parallel after p2-c003:** These two changes are
independent of each other (`forge-identity` vs `fdb-postgres`) and can
be executed simultaneously. Both depend only on `fdb-reflection` types being
defined, not on compiler implementations.

**p2-c004 after 2a+2b:** The REST compiler handlers call `fdb_auth::rls_from_bearer()`
(needs p2-c001) and `backend.acquire(rls)` (needs p2-c002). Both must be
real implementations, not stubs, for integration tests to pass.

**p2-c005 after p2-c004:** `StateManager::do_compile()` calls `RestCompiler::compile()`
— needs the compiler to be functional. The gateway wiring also requires a
working compiled `Router`.

**p2-c007 after p2-c005:** `OpenApiCompiler::compile()` result is stored in
`CompiledState.openapi_doc` which `StateManager::do_compile()` calls. Can
actually run in parallel with p2-c005 since it only needs the `DatabaseModel`
type — but ordering after p2-c005 simplifies the gateway wiring step
(both compilers wired at the same time in `fdb-gateway/src/main.rs`).

**p2-c006 last:** Vector support is additive to the RPC handler written in
p2-c004. Also blocked on OQ-9 (pgvector version check in PG18 image).

---

## Dependency Graph

```
p2-c003 (IR crate scaffold)
    ├── p2-c001 (JWT verify)      ─┐
    └── p2-c002 (pool + SET LOCAL) ─┤ (parallel)
                                    │
                                    ▼
                              p2-c004 (REST compiler)
                                    │
                                    ▼
                              p2-c005 (StateManager hot-reload)
                                    │
                              ┌─────┴──────┐
                              ▼            ▼
                         p2-c007        p2-c006
                      (OpenAPI)      (pgvector)
```

---

## Workspace Dependency Changes

These must be added to root `Cargo.toml [workspace.dependencies]` as part of
the plan execution (tracked in individual task files):

| Dep | Version | Added by |
|---|---|---|
| `sqlx` | `"0.8"` (features: postgres, runtime-tokio, uuid, json) | p2-c003 |
| `deadpool-postgres` | `"0.14"` | p2-c002 |
| `tokio-postgres` | `"0.7"` | p2-c002 |
| `jsonwebtoken` | `"9"` | p2-c001 |
| `reqwest` | `"0.12"` (features: json, rustls-tls) | p2-c001 |
| `pgvector` | `"0.4"` (features: sqlx) | p2-c006 |

---

## Security Gates (enforce at every change)

1. No JWT payload values in any `tracing` span or log output
2. `EncryptedDek(Vec<u8>)` only in `DatabaseModel` — no `plaintext_*` fields
3. `SET LOCAL` inside transaction — `Conn` must own the `Transaction`
4. Column names in SQL validated against `DatabaseModel` allowlist
5. `role` absent from JWT → coerce to `"anon"` (not an error)
6. `fdb-reflection` must NOT import `fdb-gateway` (hexagonal rule)

---

## Next Action
`/kbd-execute p2-quarry-reflection-engine`

Execute order: p2-c003 → (p2-c001 || p2-c002) → p2-c004 → p2-c005 → p2-c007 → p2-c006
