# Assessment — p3-graphql-hybrid-engine

**Date:** 2026-06-30  
**Assessor:** kbd-assess  
**Baseline:** workspace GREEN — `cargo check` clean, 9 tests pass (0 failures), clippy pedantic with `-D warnings`

---

## Phase Gate Recall

> `POST /graphql` → `graphql.resolve()` under RLS; `GET /graphql` → `graphql-transport-ws`; introspection = pg_graphql ∪ subscription SDL; Keto inline check per subscription event; per-event RLS re-query (non-negotiable).

---

## Codebase State vs. Goals

### G1 — p3-c005-pg-graphql-pg18 (OQ-3 resolution)

**Status: NOT STARTED — BLOCKER**

- `docs/contracts/` has `jwt-contract.md` and `vault-kms.md` but **no** `pg-graphql-version.md`
- `images/postgres18/Dockerfile` exists (p1-c010 from Phase 1) but the pg_graphql build step has not been verified against PG18 compatibility
- OQ-3 is listed in `current-waypoint.json` as outstanding
- **Must be resolved before p3-c001 coding begins**

Gap: `docs/contracts/pg-graphql-version.md` does not exist; pg_graphql PG18 tagged release status unverified.

---

### G2 — p3-c001-graphql-passthrough (POST /graphql → graphql.resolve())

**Status: NOT STARTED**

- `fdb-gateway/src/main.rs`:63 has `// TODO: .route("/graphql", ...)` — the `POST /graphql` route is completely absent
- `crates/fdb-postgres/src/lib.rs`:110–113 — `PgGraphQl::execute()` is `todo!("pg_graphql passthrough")` — the actual `SELECT graphql.resolve($query, $vars, $extensions)` call is not written
- `fdb-ports::GraphQlExecutor` trait is defined correctly (§3.1)
- `fdb-domain` has `GraphQlRequest` type — confirmed present

Outstanding security item from Phase 2: `forge-identity::verify_and_build()` (line 61) has no `#[instrument(skip(bearer))]` — bearer token could appear in tracing spans. **Must be closed in this change.**

Gap: No `/graphql` route registered; `PgGraphQl::execute()` body absent; `#[instrument(skip(bearer))]` missing on `verify_and_build`.

---

### G7 — p3-c007-graphql-compiler (GraphQlCompiler → CompiledState)

**Status: STUB ONLY**

- `crates/fdb-reflection/src/compilers/graphql.rs` — `GraphQlCompiler::compile()` is `todo!("Phase 3: GraphQlCompiler")` (line 9)
- `CompiledState` (`compiled.rs`) has: `version`, `database_model`, `router`, `openapi_doc` — **no `graphql_schema` or `subscription_schema` field**
- `StateManager::do_compile()` calls `RestCompiler::compile()` and `OpenApiCompiler::compile()` but **not** `GraphQlCompiler::compile()` — the subscription schema is never built or stored
- `async-graphql` is **not in any Cargo.toml** (workspace or crate-level)

Gaps:
1. `async-graphql = "7"` + `async-graphql-axum` not in `[workspace.dependencies]`
2. `CompiledState` needs a `subscription_schema: Option<async_graphql::dynamic::Schema>` (or equivalent) field
3. `GraphQlCompiler::compile()` body: must iterate `DatabaseModel.tables`, generate a dynamic async-graphql schema with `tChanges(filter)` subscriptions per table
4. `StateManager::do_compile()` must call `GraphQlCompiler::compile()` and store the schema
5. `CompiledState` does not carry the subscription schema through the hot-swap

---

### G8 — p3-c008-extended-guc-propagation

**Status: NOT STARTED**

- `crates/fdb-postgres/src/lib.rs`:44–94 — `PgBackend::acquire()` currently sets only the 3 base `SET LOCAL` statements (`ROLE`, `request.jwt.claims`, `request.headers`)
- The spec (§2.2 and Phase 3 G8) requires additional GUC propagation:
  - `SET LOCAL "app.jwt_claims" = $1` (redundant alias used by some pgrx extensions)
  - `SET LOCAL "app.keto_subject" = $1` (for Keto inline check from SQL)
  - `SET LOCAL "app.vault_key_id" = $1` (for Vault key selection per request)
- `RlsContext` struct has `role`, `claims_json`, `raw_bearer` — no `keto_subject` or `vault_key_id` fields
- `forge-identity::Claims` has `sub`, `role`, `tenant_id`, `extra` — keto_subject is derivable from `sub`; `vault_key_id` comes from JWT claims

Gaps:
1. `RlsContext` needs optional `keto_subject: Option<String>` and `vault_key_id: Option<String>` fields (or builder logic to derive from `Claims`)
2. `PgBackend::acquire()` needs 3 additional `SET LOCAL` statements inside the same `BEGIN` transaction
3. `verify_and_build()` needs to populate these derived fields from the decoded `Claims`

---

### G3 — p3-c002-subscriptions (FRF WatchEntityType + Keto + per-event RLS re-query)

**Status: STUB ONLY**

- `crates/fdb-realtime/src/lib.rs` — `FabricChangeSource::watch()` is `todo!("fabric WatchEntityType + Keto gate + per-event RLS re-query")`
- `fdb-realtime/Cargo.toml` has `# TODO(p3-c002): tonic client for fabric WatchEntityType; Keto gate` — tonic is not added
- `FabricChangeSource` struct comment lists `/* tonic channel, keto client, pg pool for re-query */` — none are populated
- Port trait `ChangeStreamSource` is correctly defined in `fdb-ports`
- `fdb-domain` has `ChangeEvent`, `SubscriptionSpec` — confirmed present

External dependency: **FRF `WatchEntityType` gRPC service** — requires the FRF `.proto` file or generated client. This is OQ-1 from `current-waypoint.json`.

Gaps:
1. `tonic` + FRF proto client not in `fdb-realtime/Cargo.toml`
2. Keto check client not in `fdb-realtime/Cargo.toml` (likely `ory-client` or direct HTTP to Keto)
3. `FabricChangeSource` struct fields all absent
4. The per-event RLS re-query pool (`PgPool` or `DatabaseBackend`) not wired
5. The non-negotiable WAL bypass re-query logic (`SELECT * FROM <schema>.<table> WHERE pk = $1` under subscriber `RlsContext`) is entirely unwritten

---

### G4 — p3-c004-graphql-transport-ws (WebSocket upgrade)

**Status: NOT STARTED**

- `fdb-gateway/src/main.rs` has no WebSocket route or handler
- `async-graphql-axum` not in any Cargo.toml
- `graphql-transport-ws` protocol not mentioned in any Cargo.toml
- `async-graphql` `GraphQLSubscription` service not wired

Gaps:
1. `async-graphql-axum` dependency absent
2. No `GET /graphql` route with WebSocket upgrade handler
3. `GraphQLSubscription` service not composed with `FabricChangeSource`

---

### G5 — p3-c003-introspection-merge

**Status: NOT STARTED**

- No introspection handler in `fdb-gateway`
- `__schema`/`__type` resolution not present in `PgGraphQl` or gateway
- No SDL merge logic exists anywhere in the codebase
- pg_graphql introspection is proxied in-DB; subscription SDL would come from the dynamic async-graphql schema built in G7

Gaps:
1. No introspection handler or merge route
2. SDL union logic (pg_graphql introspection ∪ async-graphql subscription SDL) not designed or implemented
3. Requires G7 (GraphQlCompiler) to exist first — blocked

---

### G6 — p3-c006-keto-sync (FRF Iggy → flint_meta.keto_tuples)

**Status: NOT STARTED**

- `ext-flint-meta/src/keto.rs` has `flint_meta.keto_tuples` table DDL from Phase 1
- No FRF Iggy consumer exists in any Rust crate
- `check_permission()` exists in `flint_meta` (SQL) from Phase 1 — the SQL side is in place
- The Rust-side `flint_meta.check_permission()` inline call from `FabricChangeSource` does not exist

External dependency: **FRF Iggy `keto_changes` event type** — OQ-8 from `current-waypoint.json`, unresolved.

Gaps:
1. No Iggy consumer crate / no `iggy` client dependency anywhere
2. The `keto_changes` event type from FRF is unverified (OQ-8)
3. `FabricChangeSource` does not call `flint_meta.check_permission()` as part of the subscription path (connected to G3 gap)

---

## Dependency Verification

| Dependency | Status | Notes |
|------------|--------|-------|
| `fdb-reflection::DatabaseModel` | **PRESENT** | Full model with tables/columns/fns/views |
| `fdb-reflection::CompiledState` | **PRESENT** | Needs `subscription_schema` field added |
| `fdb-reflection::StateManager` | **PRESENT** | ArcSwap hot-reload working; needs GraphQL schema added to compile |
| `fdb-postgres::PgBackend::acquire()` | **PRESENT** | 3 SET LOCAL statements; needs 3 more for G8 |
| `forge-identity::RlsContext` | **PRESENT** | Needs `keto_subject`, `vault_key_id` for G8 |
| `forge-identity::verify_and_build()` | **PRESENT** | Missing `#[instrument(skip(bearer))]` |
| `fdb-ports::GraphQlExecutor` | **PRESENT** | Trait seam correct |
| `fdb-ports::ChangeStreamSource` | **PRESENT** | Trait seam correct |
| `fdb-domain::GraphQlRequest` | **PRESENT** | Confirmed |
| `fdb-domain::ChangeEvent` | **PRESENT** | Confirmed |
| `flint_meta.keto_tuples` (SQL) | **PRESENT** (Phase 1) | DDL installed |
| `flint_meta.check_permission()` (SQL) | **PRESENT** (Phase 1) | Needs Rust caller |
| `async-graphql = "7"` | **ABSENT** | Not in workspace deps |
| `async-graphql-axum` | **ABSENT** | Not in any Cargo.toml |
| `tonic` (fabric client) | **ABSENT** | Not in workspace deps |
| FRF WatchEntityType `.proto` | **UNKNOWN** | Need to check `flint-realtime-fabric` repo |
| pg_graphql PG18 release | **UNKNOWN** | OQ-3, pre-kickoff blocker |
| FRF Iggy `keto_changes` event | **UNKNOWN** | OQ-8 |

---

## Security Contract Gaps

| Contract | Status | Gap |
|----------|--------|-----|
| `#[instrument(skip(bearer))]` on `verify_and_build()` | **MISSING** | Line 61 of `forge-identity/src/lib.rs` — `bearer` may appear in tracing spans |
| Per-event RLS re-query (WAL bypass) | **NOT IMPLEMENTED** | `FabricChangeSource::watch()` is `todo!()` |
| Keto check per subscription event | **NOT IMPLEMENTED** | No Keto client in `fdb-realtime` |
| Extended GUC propagation inside `BEGIN` transaction | **NOT IMPLEMENTED** | `PgBackend::acquire()` missing 3 SET LOCAL statements |
| No JWT payload in `/graphql` handler tracing | **NOT YET APPLICABLE** | Handler doesn't exist yet; must be enforced on creation |

---

## Phase 2 Security Items to Close

These were flagged in Phase 2 `reflection.md` and MUST be closed in Phase 3 (earliest applicable change):

1. **`#[instrument(skip(bearer))]` on `verify_and_build()`** — close in `p3-c008` (extends `acquire()` context) or standalone fix before `p3-c001`
2. **Column-name SQL injection validation gate test** — write in `p3-c001` REST handler bodies or `p3-c007` GraphQL compiler (guard against injected table/column names in `SELECT graphql.resolve()` call)

---

## Open Questions (Updated)

| OQ | Question | Status | Blocks |
|----|----------|--------|--------|
| OQ-3 | pg_graphql PG18 tagged release? | **UNRESOLVED — PRE-KICKOFF GATE** | p3-c005, p3-c001 |
| OQ-8 | FRF Iggy `keto_changes` event type available? | **UNRESOLVED** | p3-c006 |
| OQ-FRF-1 | FRF WatchEntityType `.proto` location? | **NEW — check `flint-realtime-fabric` repo** | p3-c002 |
| OQ-FRF-2 | FRF tonic client: already generated or generate from proto? | **NEW** | p3-c002 |
| OQ-GQL-1 | async-graphql "dynamic schema" API in v7 sufficient for subscription SDL generation? | **NEW — confirm via Context7** | p3-c007 |

---

## Change Readiness

| Change | Readiness | Blocker |
|--------|-----------|---------|
| p3-c005-pg-graphql-pg18 | READY to research | OQ-3 resolution (web research required) |
| p3-c008-extended-guc-propagation | READY to implement | None — pure Rust, no external deps |
| p3-c001-graphql-passthrough | BLOCKED | OQ-3 must resolve first; `#[instrument(skip(bearer))]` fix first |
| p3-c007-graphql-compiler | BLOCKED on dep | `async-graphql = "7"` must be added to workspace first |
| p3-c002-subscriptions | BLOCKED on dep | `tonic` + FRF proto + OQ-FRF-1 must resolve |
| p3-c004-graphql-transport-ws | BLOCKED on dep | `async-graphql-axum` + G7 (GraphQlCompiler) must exist |
| p3-c003-introspection-merge | BLOCKED on impl | G7 must exist first |
| p3-c006-keto-sync | BLOCKED on OQ | OQ-8 (FRF Iggy event type) |
| p3-c009-predicate-pushdown | DEFERRED | P2; implement after core subscription works |

---

## Recommended Execution Order

```
p3-c005  → verify pg_graphql PG18 (research only, writes docs/contracts/pg-graphql-version.md)
p3-c008  → extend SET LOCAL block + RlsContext fields + fix #[instrument(skip(bearer))]
p3-c001  → POST /graphql route + PgGraphQl::execute() body + graphql.resolve() passthrough
p3-c007  → add async-graphql to workspace + GraphQlCompiler + CompiledState schema field
p3-c004  → GET /graphql WebSocket + async-graphql-axum + GraphQLSubscription service
p3-c002  → FabricChangeSource: tonic + Keto + per-event RLS re-query
p3-c003  → introspection merge (pg_graphql ∪ subscription SDL)
p3-c006  → Iggy keto_changes sync (after OQ-8 resolved)
```

The first two changes (p3-c005 and p3-c008) are unblocked and can start immediately.

---

## Workspace Build Health at Assessment Time

| Metric | Value |
|--------|-------|
| `cargo check --workspace` | GREEN |
| `cargo test --workspace` | 9 pass, 0 fail |
| `cargo clippy --workspace -- -D warnings` | (not run at assess time; run at each change completion) |
| Target crates with `todo!()` in Phase 3 scope | `fdb-gateway/main.rs`, `fdb-postgres/lib.rs` (PgGraphQl), `fdb-realtime/lib.rs`, `fdb-reflection/compilers/graphql.rs` |
| Phase 3 Cargo deps absent | `async-graphql`, `async-graphql-axum`, `tonic` |
