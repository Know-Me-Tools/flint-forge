# Goals — p3-graphql-hybrid-engine

## Phase Gate

`POST /graphql` handles Query and Mutation via `graphql.resolve()` under RLS; `GET /graphql` upgrades to `graphql-transport-ws` for Subscriptions via async-graphql + FRF `WatchEntityType`; introspection merges pg_graphql schema ∪ subscription SDL; Keto inline permission check works end-to-end for subscription events; per-event RLS re-query contract enforced (WAL bypass is non-negotiable).

## Pre-Kickoff Gate (MUST resolve before any coding)

- **OQ-3 RESOLUTION REQUIRED:** Verify `supabase/pg_graphql` has a tagged release supporting Postgres 18. If not released, build from master SHA and pin in `docs/contracts/pg-graphql-version.md`. Do NOT start p3-c001 until OQ-3 is resolved.

## Functional Goals

- **G1** — `p3-c005-pg-graphql-pg18`: OQ-3 resolved and documented in `docs/contracts/pg-graphql-version.md`; Dockerfile confirmed or updated with correct pg_graphql build
- **G2** — `p3-c001-graphql-passthrough`: `POST /graphql` → `SELECT graphql.resolve($query, $variables, $extensions)` under full RLS context (`SET LOCAL` block from p2-c002 + extended GUC propagation from G8); response JSON returned verbatim; async-graphql is NOT in this path
- **G3** — `p3-c002-subscriptions`: `fdb-realtime` gRPC client → FRF `WatchEntityType`; for each `EntityChange` event, re-query the changed row under subscriber's `RlsContext` before delivery (WAL bypass safety); Keto gate via `flint_meta.check_permission()` on each event; deliver only RLS-visible rows
- **G4** — `p3-c004-graphql-transport-ws`: `GET /graphql` WebSocket upgrade via `graphql-transport-ws` on `async-graphql-axum`; `GraphQLSubscription` service wired to `ChangeStreamSource`
- **G5** — `p3-c003-introspection-merge`: `__schema` / `__type` returns merged union of pg_graphql introspection ∪ sibling subscription SDL; the two schemas share type names without collision
- **G6** — `p3-c006-keto-sync`: FRF Iggy → `flint_meta.keto_tuples` sync; `keto_changes` NOTIFY channel integration; `check_permission()` inline in subscription delivery path
- **G7** — `p3-c007-graphql-compiler`: `GraphQlCompiler::compile(model: &DatabaseModel) -> async_graphql::Schema`; integrated into `CompiledState` hot-swap in `StateManager`; sibling subscription schema rebuilt on DDL change
- **G8** — `p3-c008-extended-guc-propagation`: Add `app.jwt_claims`, `app.keto_subject`, `app.vault_key_id` to `SET LOCAL` block in `PgBackend::acquire()`; extend `RlsContext` if needed

## Security Contracts (Non-Negotiable)

- **Subscription RLS enforcement (CLAUDE.md §Subscription RLS Enforcement):** WAL bypasses RLS. For EVERY `EntityChange` from FRF, Quarry re-queries `SELECT * FROM <schema>.<table> WHERE id = $1` under the subscriber's full `RlsContext`. The row is delivered ONLY if the re-query returns a row. This is mandatory and not configurable. Predicate-pushdown (p3-c009) is off by default and requires explicit operator opt-in with acknowledged data-leak risk.
- **No JWT payload logging:** All `RlsContext` fields (`role`, `claims_json`, `raw_bearer`) are `skip`-instrumented in tracing spans. The Phase 2 gap (missing `#[instrument(skip(bearer))]` in `verify_and_build`) must be closed in G2.
- **Keto inline check:** `flint_meta.check_permission()` is called per subscription event, not per compile. It must never cache permission results across different subscriber sessions.
- **Extended GUC propagation** (`app.jwt_claims`, `app.keto_subject`, `app.vault_key_id`) must also be inside the same `BEGIN` transaction as the rest of the `SET LOCAL` block.

## OpenSpec Changes

| Priority | Change ID | Description |
|----------|-----------|-------------|
| **P0** | `p3-c005-pg-graphql-pg18` | OQ-3 resolution — verify/pin pg_graphql PG18 release |
| **P0** | `p3-c008-extended-guc-propagation` | Extend `SET LOCAL` block: `app.jwt_claims`, `app.keto_subject`, `app.vault_key_id` |
| **P0** | `p3-c001-graphql-passthrough` | `POST /graphql` → `graphql.resolve()` under RLS |
| **P0** | `p3-c007-graphql-compiler` | `GraphQlCompiler` → `CompiledState` hot-swap |
| **P0** | `p3-c002-subscriptions` | FRF WatchEntityType + Keto gate + per-event RLS re-query |
| **P0** | `p3-c004-graphql-transport-ws` | `graphql-transport-ws` WebSocket upgrade |
| **P0** | `p3-c003-introspection-merge` | Union pg_graphql ∪ subscription SDL |
| **P0** | `p3-c006-keto-sync` | FRF Iggy → `flint_meta.keto_tuples` sync |
| P2 | `p3-c009-predicate-pushdown` | Opt-in RLS predicate pushdown (off by default) |

MVP = p3-c005 + p3-c008 + p3-c001 + p3-c007 + p3-c002 + p3-c004 + p3-c003 + p3-c006

## Dependencies

### From Phase 2 (all must be present)
- `fdb-reflection`: `DatabaseModel`, `CompiledState`, `StateManager`, `ReflectionEngine` — **DELIVERED** (p2-c003)
- `fdb-postgres`: `PgBackend::acquire()` with `SET LOCAL` RLS block — **DELIVERED** (p2-c002)
- `forge-identity`: `RlsContext`, async `verify_and_build()` — **DELIVERED** (p2-c001)
- `StateManager::start_listener()` ArcSwap hot-reload loop — **DELIVERED** (p2-c005)
- `GraphQlCompiler` stub in `fdb-reflection/src/compilers/graphql.rs` — **DELIVERED** (stub, p2-c003)

### From Phase 1
- `flint_meta.keto_tuples` table + `check_permission()` function — **DELIVERED** (p1-c009)
- `flint_meta` DDL triggers → `pg_notify('meta_runtime', ...)` — **DELIVERED** (p1-c008)

### External / FRF
- FRF `WatchEntityType` gRPC service — required for p3-c002; FRF Phase 1 gate
- FRF Iggy event bus — required for p3-c006; FRF Phase 3

### Open Questions to Resolve at Kickoff
| OQ | Question | Blocks |
|----|----------|--------|
| OQ-3 | pg_graphql PG18 tagged release available? | p3-c001, p3-c005 |
| OQ-8 | Keto sync via FRF Iggy: `keto_changes` event type available? | p3-c006 |

## Carries Forward from Phase 2

Phase 2 left two outstanding security items that MUST be closed in Phase 3:
1. Missing `#[instrument(skip(bearer))]` in `forge-identity::verify_and_build()` — close in G2/G8
2. Column-name SQL injection validation gate test — close in G2 (not REST, but same pattern applies to GraphQL variable injection)

## Phase 2 P1 Backfill (can run parallel to Phase 3 prep)
- `p2-c006-pgvector-rpc` — pgvector `/rpc` vector similarity (blocks Phase 5)
- `p2-c007-openapi-compiler` — OpenAPI compiler + `GET /openapi.json`

## Enables

- Phase 4 (Flint Ember): LLM/embedding in-DB needs `CompiledState` with GraphQL schema
- Phase 5 (A2UI Registry / Kiln): `CompiledState.agui_descriptors` from `flint_a2ui` (p5-c009 builds on Phase 3 Keto sync)
- Phase 7 (AG-UI / MCP): `McpCompiler` and AG-UI emitter extend `fdb-reflection` → needs Phase 3 `GraphQlCompiler`
