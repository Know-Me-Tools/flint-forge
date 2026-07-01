# Goals — p2-quarry-reflection-engine

## Phase Gate

`fdb-gateway` serves live REST requests against a real Postgres table via `DatabaseModel` + `RestCompiler`; schema changes hot-swap via `StateManager` (ArcSwap + PgListener) without restarting the process; `/openapi.json` reflects the live schema; all JWT/RLS context propagated per-request.

## Functional Goals

- **G1** — `fdb-auth` crate: JWT verify → `RlsContext` (`forge-identity::verify_and_build`); correct treatment of the `role` claim (MUST be added to `additional_claims` by caller — never auto-included)
- **G2** — `fdb-postgres` adapter: deadpool-postgres pool; per-request `SET LOCAL ROLE`, `SET LOCAL "request.jwt.claims"`, `SET LOCAL "request.headers"` before any user statement
- **G3** — `fdb-reflection` crate: `DatabaseModel` IR compiled from `flint_meta.*` tables; `CompiledState` struct (`Arc<DatabaseModel>` + `ArcSwap<Router<()>>` + `agui_descriptors`); `ReflectionEngine::reflect()` calling all `flint_meta` SQL functions
- **G4** — `RestCompiler`: `DatabaseModel` → `axum::Router<()>`; HTTP → SQL AST → parameterized query; full CRUD (`GET`/`POST`/`PATCH`/`DELETE`) + `/rpc`; Range headers for pagination; no string concatenation in SQL paths
- **G5** — `StateManager::start_listener()`: `sqlx::PgListener` on `meta_runtime` channel → recompile → `ArcSwap::store()`; reconnect loop (sqlx PgListener has no auto-reconnect); no dropped in-flight requests during swap
- **G6** — `OpenApiCompiler`: `DatabaseModel` → `utoipa::openapi::OpenApi`; live at `GET /openapi.json`; reflects current hot-swapped schema
- **G7** — Gate tests pass: concurrent load during hot-swap; all filter operators covered (`eq`, `neq`, `gt`, `gte`, `lt`, `lte`, `like`, `ilike`, `in`, `is`, `cs`, `cd`); Vault DEK ciphertext-only in `CompiledState` (no plaintext in memory beyond single request)

## Critical Security Contracts Carried Forward

- Plaintext DEK MUST NOT be stored in `CompiledState` — `dek_encrypted` (ciphertext only) lives in `DatabaseModel`; decryption happens per-request via `flint_meta.decrypt_column()`
- `role` claim is NOT auto-included in JWTs minted by flint-gate — every authenticated route MUST add `role` to `additional_claims`; documented in `docs/contracts/jwt-contract.md`

## OpenSpec Changes

| Priority | Change | Description |
|----------|--------|-------------|
| P0 | `p2-c001-fdb-auth` | JWT verify → `RlsContext` |
| P0 | `p2-c002-fdb-postgres` | deadpool pool + SET LOCAL RLS context |
| **P0** | `p2-c003-flint-reflection-crate` | `fdb-reflection`: `DatabaseModel`, `CompiledState`, `StateManager`, `ReflectionEngine` |
| **P0** | `p2-c004-rest-compiler` | `RestCompiler`: HTTP → SQL AST → parameterized query, full CRUD |
| **P0** | `p2-c005-arcswap-hot-reload` | `StateManager` PgListener hot-swap loop |
| P1 | `p2-c006-pgvector-rpc` | `/rpc/<fn>` vector similarity via `embedding <-> $q` |
| P1 | `p2-c007-openapi-compiler` | `OpenApiCompiler` → `/openapi.json` |

MVP = p2-c001 + p2-c002 + p2-c003 + p2-c004 + p2-c005 (live hot-swapping REST server)

## Dependencies on Phase 1

- `flint_meta` schema (p1-c007): all `flint_meta.*` tables must exist
- `flint_meta` DDL triggers (p1-c008): `pg_notify('meta_runtime', ...)` on schema changes
- `flint_meta` SQL functions (p1-c009): `tables()`, `columns()`, `relationships()`, `functions()`, `version()`, `check_permission()`, `set_identity()`
- `agui_descriptor()` (p1-c010): initial form of A2UI descriptor (will be corrected in p5-c009)
- JWT contract (p1-c005): role-claim shape pinned and documented

## Enables

- Phase 3: GraphQL compiler needs `CompiledState` / `DatabaseModel` IR
- Phase 5 p5-c009: `CompiledState.agui_descriptors` upgrade to query `flint_a2ui`
- Phase 7 p7-c003/p7-c004: AG-UI emitter + MCP compiler both extend `fdb-reflection`
