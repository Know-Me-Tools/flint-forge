# Assessment — p3-auth-rls-keto

**Date:** 2026-07-01  
**Status:** Complete  
**Inspector:** kbd-assess

---

## Executive Summary

Phase 3 inherits a stronger foundation than expected. The gateway, Keto sync, GraphQL hybrid routing, and extended GUC propagation are all materially delivered. The major outstanding work is: Cedar policy engine (new crate, no implementation), CRUD handler bodies (four `todo!()` stubs), and subscription RLS enforcement (scaffolded but blocked on FRF WatchEntityType RPC, OQ-FRF-1).

**Pre-certified as DONE (no Phase 3 change needed):**
- `p3-c008`: 6 SET LOCAL GUCs in `fdb-postgres::acquire()` including `app.jwt_claims`, `app.keto_subject`, `app.vault_key_id`
- Keto sync background task: `keto_sync.rs` (254 lines, full impl + unit tests) wired in `fdb-gateway/src/main.rs`
- GraphQL WebSocket upgrade: `graphql_ws_handler()` in gateway
- pg_graphql passthrough for Query/Mutation: `handle_graphql_query()` in gateway
- IntrospectionMerger stub: called in gateway (`IntrospectionMerger::merge()`)
- GraphQL compiler (subscription schema): `fdb-reflection/src/compilers/graphql.rs` — full `GraphQlCompiler::compile()` with async_graphql dynamic schema, per-table subscription fields, tests

**Outstanding work (true Phase 3 deliverables):**

| # | Area | Gap | Size |
|---|------|-----|------|
| 1 | `forge-policy` Cedar | Trait only, no `cedar-policy` crate, no impl | LARGE |
| 2 | Keto `KetoCacheClient` in `fdb-app` | HTTP check is in `fdb-realtime`, not integrated into use-cases | MEDIUM |
| 3 | CRUD handler bodies | All 4 `todo!()` in `RestCompiler` | LARGE |
| 4 | Subscription RLS enforcement | Scaffolded in `fdb-realtime`, blocked on OQ-FRF-1 | MEDIUM (blocked) |
| 5 | Gate tests (G6) | No test files for SQL injection validation, DEK serde, subscription RLS | LARGE |
| 6 | `fdb-realtime` WatchEntityType gRPC | Empty stream placeholder, OQ-FRF-1 | MEDIUM (blocked) |

---

## Goal-by-Goal Gap Analysis

### G1 — `forge-policy` Cedar Policy Engine

**Status: LARGE GAP — new crate work required**

**What exists:**
- `forge-policy/src/lib.rs`: `Pep` trait, `Decision` enum (`Allow`/`Deny`), `Request` struct (`action`, `resource`, `context`)
- `forge-policy/Cargo.toml`: depends on `forge-domain`, `forge-identity`, `async-trait`, `thiserror` — no `cedar-policy` crate

**What's missing:**
- `cedar-policy = "4"` (or current) in `forge-policy/Cargo.toml`
- `CedarPolicyEngine` struct implementing `Pep`
- Policy bundle loading from `flint_meta.cedar_policies` table (requires sqlx + pg integration)
- Policy schema compilation and caching
- `PolicyEngine::evaluate(principal, action, resource, context) -> Decision` implementation
- Integration into `fdb-app` mutation use-cases

**Security note:** Cedar policies must be loaded from the privileged pool (not the RLS pool). Policy loading failure must fail closed (`Decision::Deny`).

**Dependency note:** `flint_meta.cedar_policies` table must exist (check Phase 1 migrations before starting).

---

### G2 — Keto Coarse Relationship Check Integration

**Status: PARTIAL — HTTP check exists, not wired into app layer**

**What exists:**
- `fdb-realtime/src/lib.rs`: `keto_check_via_http()` — calls Keto HTTP API, fails closed on unavailability
- `keto_sync.rs`: full `KetoCacheClient` polling `flint_meta.keto_tuples`, `cache_check()` function, `KetoCache` type alias
- Both implementations are present but in different crates

**What's missing:**
- `KetoCacheClient` type is not yet wired into `fdb-app` use-cases for mutation gating
- `fdb-app` does not import `KetoCache` from `fdb-gateway::keto_sync`
  - **Hexagonal violation risk**: `fdb-app` cannot import `fdb-gateway`. The cache must be provided via a port trait (inject `Arc<dyn KetoCheck>` into `Quarry`) or pass `KetoCache` as a parameter at composition time in the gateway
- No `KetoCacheClient` newtype — just the raw `KetoCache = Arc<RwLock<Vec<KetoCacheEntry>>>`
- No TTL on individual cache entries (the whole cache is refreshed on poll, acceptable)
- No integration test for Keto-gated mutation path

**Design note for p3-c002:** The correct seam is to define a `KetoCheck` trait in `fdb-ports`, implement it in `keto_sync.rs` using `cache_check()`, and inject it into `Quarry` at composition time. This avoids the hexagonal violation.

---

### G3 — CRUD Handler Bodies in `RestCompiler`

**Status: LARGE GAP — all four handlers are `todo!()`**

**What exists:**
- `fdb-reflection/src/compilers/rest.rs`: route registration complete, `handle_rpc` fully implemented (pgvector dispatch), `handle_list`/`handle_insert`/`handle_update`/`handle_delete` return `todo!()`

**What's missing (each handler):**

`handle_list`:
- Filter operator dispatch (12 operators: eq, neq, gt, gte, lt, lte, like, ilike, in, is, cs, cd)
- `is_safe_identifier()` validation on all column names before SQL interpolation
- `SELECT ... WHERE` construction with parameterized values
- Range header pagination (`Range: rows=0-24` → `LIMIT 25 OFFSET 0`)
- `Content-Range` response header
- RLS context already set by `acquire()` — no additional GUC work needed

`handle_insert`:
- Parse JSON body into column/value pairs
- Column name safety validation (same `is_safe_identifier()` gate)
- `INSERT INTO ... (cols) VALUES ($1, $2, ...) RETURNING *`
- 201 response with `Location` header

`handle_update`:
- Parse query params for filter (same operator dispatch)
- Parse JSON body for SET values
- `UPDATE ... SET ... WHERE ...` with parameterized values
- 200 or 204 response

`handle_delete`:
- Parse query params for filter
- `DELETE FROM ... WHERE ...` with parameterized values
- 204 response

**Critical security gate:** `is_safe_identifier()` must be called on EVERY table name and column name before interpolation. This is the P2 security gate that was never tested (`test_rest_select_with_eq_filter` was marked `#[ignore]`).

**Existing validator:** Check if `is_safe_identifier()` already exists in `fdb-reflection` or `forge-domain` before reimplementing.

---

### G4 — GraphQL Hybrid

**Status: MOSTLY DONE — pre-certified, minor wiring gap**

**What exists:**
- `fdb-gateway/src/main.rs`:
  - `graphql_ws_handler()`: WebSocket upgrade using `GraphQLWebSocket::new(socket, schema, protocol)` — pulls `subscription_schema` from `CompiledState`
  - `handle_graphql_query()`: pg_graphql passthrough with `IntrospectionMerger::merge()` for subscription types
- `fdb-reflection/src/compilers/graphql.rs`: `GraphQlCompiler::compile()` — builds async_graphql dynamic `Schema` with per-table `<TableName>Changes` subscription fields

**What's missing (only):**
- The reflection router is not yet nested in the gateway (`TODO(p2-c005)` comment in `main.rs` — reflection routes not mounted yet)
- Subscription resolver stub returns empty stream (depends on G7/OQ-FRF-1 for real data)
- `IntrospectionMerger` may be a stub — verify it correctly merges pg_graphql schema SDL with subscription SDL

**OQ-3 pre-flight:** Run `SELECT extversion FROM pg_extension WHERE extname = 'pg_graphql';` against PG18 container before implementing G4. If pg_graphql not present, the passthrough path errors — need a stub or install step.

---

### G5 — Subscription RLS Enforcement

**Status: SCAFFOLDED — structure correct, blocked on OQ-FRF-1**

**What exists in `fdb-realtime/src/lib.rs`:**
- `FabricChangeSource` struct with `channel: Arc<tonic::Channel>`, `http: reqwest::Client`, `keto: KetoConfig`
- `keto_check_via_http()`: Keto check before subscribing, fails closed
- `watch()`: runs Keto check, returns empty stream with `warn!` log (OQ-FRF-1 placeholder)
- Commented-out implementation shows the full pattern: tonic WatchEntityType → `filter_map` → `rls_requery` per event

**What's missing:**
- Real `WatchEntityType` tonic client call (OQ-FRF-1 blocks this)
- `rls_requery()` function: re-queries the changed row using the subscriber's `RlsContext` — this is the non-negotiable security invariant
- Integration test: `test_subscription_rls_drops_unauthorized_events` (G6)

**Design note:** The `rls_requery` must use a fresh connection from the RLS pool with full `acquire()` GUC setup, not the privileged pool. If the re-query returns zero rows, the event is silently dropped (not errored — the subscriber simply doesn't receive it). This is correct behavior.

**OQ-FRF-1 status:** FRF `WatchEntityType` RPC not yet exposed. The empty-stream placeholder is intentional and correct for now. G7/G5 are linked — G5 can only be fully complete when G7 unblocks.

---

### G6 — Gate Tests

**Status: NOT STARTED**

Four gate tests required, none exist:

1. **`test_rest_select_with_eq_filter`** — carried from P2 security gate, must cover all 12 filter operators AND verify `is_safe_identifier()` blocks injection attempts
2. **`test_vault_dek_not_in_compiled_state`** — carried from P2 DEK serde gate, verifies `CompiledState` JSON serialization does not emit any `vault_key` or plaintext DEK field
3. **`test_subscription_rls_drops_unauthorized_events`** — unit test with mock `ChangeStreamSource`; subscriber with restricted RLS sees zero events for rows they don't own
4. **`test_keto_check_gates_mutation`** — unit test verifying mutation use-case returns 403 when `KetoCheck::check()` returns `false`

Tests 3 and 4 can be fully implemented before OQ-FRF-1 resolves using mocks.
Tests 1 and 2 depend on G3 handler bodies (test 1) and `CompiledState` serialization (test 2).

---

### G7 — `fdb-realtime` WatchEntityType gRPC Client

**Status: SCAFFOLDED — blocked on OQ-FRF-1**

**What exists:**
- Tonic channel already constructed (`Arc<tonic::Channel>`)
- Commented-out `watch()` implementation showing the full tonic call pattern
- `frf_op_to_domain()` mapping `i32 → ChangeOp`

**What's missing:**
- FRF `WatchEntityType` proto definition / generated client code — OQ-FRF-1
- Reconnect loop (exponential backoff on stream disconnect)
- Service token auth (header injection on tonic channel)
- Fan-out to multiple subscriber streams

**Recommendation:** Defer G7 to a sub-phase (p3-g7-stub) and deliver the empty-stream production stub with proper reconnect scaffolding. Full implementation follows when FRF team delivers the RPC.

---

## Dependency Gap Analysis

### Missing from `Cargo.toml`

| Crate | Version | Needed for | Priority |
|-------|---------|-----------|----------|
| `cedar-policy` | `"4"` | G1 `forge-policy` Cedar impl | HIGH |
| `cedar-policy-core` | (transitive) | G1 | MEDIUM |

All other dependencies appear present (tonic, async-graphql, deadpool-postgres, axum 0.8.8, sqlx, reqwest, tokio).

### Schema Pre-flights

| Table | Needed for | Status |
|-------|-----------|--------|
| `flint_meta.cedar_policies` | G1 policy loading | Unknown — check P1 migrations |
| `flint_meta.keto_tuples` | G2 / already in `keto_sync.rs` | Assumed present (keto_sync queries it) |

---

## Current Gateway Gap

One critical gap in `fdb-gateway/src/main.rs`:

- **Reflection router not mounted**: `TODO(p2-c005)` comment — the `StateManager`/`ArcSwap` is initialized but the reflection router (`fdb-reflection` REST routes) is not nested under the Axum router. This means CRUD requests currently return 404. This is the **first change to make in Phase 3** (p3-c001).

---

## Risk Register

| Risk | Severity | Mitigation |
|------|----------|-----------|
| OQ-FRF-1: WatchEntityType RPC never delivered | HIGH | Deliver G7 as production stub with reconnect scaffolding; mark G7 as conditional in phase gate |
| OQ-3: pg_graphql not available on PG18 image | MEDIUM | Pre-flight check; stub passthrough with 501 if absent |
| Cedar policy crate API churn (v3 → v4 migration) | MEDIUM | Pin `cedar-policy = "4"`, test against cargo doc; avoid internal APIs |
| `flint_meta.cedar_policies` table missing | MEDIUM | Check P1 migrations; add table in p3-c001 if absent |
| SQL injection via column names | CRITICAL | `is_safe_identifier()` must be called before EVERY SQL interpolation in G3 |
| DEK leaking through `CompiledState` serde | CRITICAL | `test_vault_dek_not_in_compiled_state` gate test (G6) |
| Hexagonal violation: `fdb-app` importing `fdb-gateway` | HIGH | Define `KetoCheck` trait in `fdb-ports`; inject at composition time |

---

## Recommended Change Order for `/kbd-plan`

1. **p3-c001** — Mount reflection router in gateway (unblocks all REST testing); check `flint_meta.cedar_policies` table
2. **p3-c002** — Define `KetoCheck` port trait; wire `KetoCacheClient` into `fdb-app` use-cases
3. **p3-c003** — `forge-policy` Cedar implementation (add `cedar-policy` crate; implement `CedarPolicyEngine`)
4. **p3-c004** — CRUD handler bodies: `handle_list` with filter operators + `is_safe_identifier()`
5. **p3-c005** — CRUD handler bodies: `handle_insert`, `handle_update`, `handle_delete`
6. **p3-c006** — Gate tests: `test_rest_select_with_eq_filter` (all 12 operators + injection guard), `test_vault_dek_not_in_compiled_state`
7. **p3-c007** — Gate tests: `test_subscription_rls_drops_unauthorized_events`, `test_keto_check_gates_mutation` (mock-based, no OQ-FRF-1 dependency)
8. **p3-c008** — ALREADY DONE (6 GUC SET LOCAL) — skip
9. **p3-c009** — `fdb-realtime` production stub: reconnect loop + service token auth (OQ-FRF-1 conditional)
10. **p3-c010** — IntrospectionMerger: verify pg_graphql SDL ∪ subscription SDL merge (OQ-3 dependent)

**Total estimated changes: 9 (2 blocked on external dependencies)**

---

## Open Questions Requiring Resolution

| OQ | Question | Gates |
|----|---------|-------|
| OQ-FRF-1 | FRF `WatchEntityType` RPC delivery timeline | G7, G5 (full implementation) |
| OQ-3 | pg_graphql PG18 tagged release | G4 passthrough path |
| OQ-cedar | `cedar-policy` crate current stable version (3 vs 4?) | G1 |
| OQ-cedar-policies-table | Does `flint_meta.cedar_policies` exist in Phase 1 migrations? | G1 |

---

## Assessment Verdict

Phase 3 is well-positioned. The gateway scaffold, Keto sync, GraphQL hybrid skeleton, and GUC propagation eliminate several planned changes. The real work is Cedar (new), CRUD bodies (medium-large), and gate tests (medium). OQ-FRF-1 creates a bounded conditional: G7 and the subscription RLS live path are deferred, but everything else can ship end-to-end.

**Phase gate confidence: ACHIEVABLE** with 8–10 focused changes. Blocked items (G7, G5 live path) are clearly bounded and do not prevent the 4-layer auth stack from being demonstrable with Keto (cache), RLS (SET LOCAL), and Cedar (forge-policy) all live.
