---
type: Reference
id: p3-c019-core-complete-with-fdb-query-rest-filter-bridge
title: p3-c019 Core Complete with fdb-query REST Filter Bridge
tags:
- flint-forge
- auth-rls
- postgrest
- fdb-query
- fdb-reflection
- keto
- phase-tracking
links:
- p3-auth-rls-keto-goals-and-compile-economy-build-config
- g4-subscription-seam-pr-and-pgrest-listen-follow-up-decisions
- p3-c019-postgrest-query-engine-draft-pr-and-t7-follow-up
sources:
- stdin
- manual:Flint Forge/p3-auth-rls-keto
timestamp: 2026-07-03T16:24:07.158883+00:00
created_at: 2026-07-03T16:24:07.158883+00:00
updated_at: 2026-07-03T16:24:07.158883+00:00
revision: 0
---

## Phase Context

- Project: Flint Forge
- Phase: `p3-auth-rls-keto`
- Status: `in_progress`
- Progress marker: `changes 7/9`
- Captured at: `2026-07-03T16:17:14Z`
- KBD root: `/Users/gqadonis/Projects/prometheus/flint-forge`

The phase gate remains the end-to-end auth/RLS/Keto/Cedar requirement described in [p3-auth-rls-keto Goals and Compile Economy Build Config](/p3-auth-rls-keto-goals-and-compile-economy-build-config.md): real `flint-gate` JWT propagation into Postgres RLS, Keto mutation gating, Cedar capability authorization, no plaintext credentials in logs or tracing spans, and parameterized SQL in CRUD handlers.

## Phase Goals

- **G1 — `forge-policy` Cedar evaluation**: `PolicyEngine::evaluate(principal, action, resource, context)` returns allow/deny; policy bundles load from `flint_meta.cedar_policies`.
- **G2 — Keto relationship checks**: coarse checks at subscribe-time and mutation-time; `KetoCacheClient` caches relation tuples with TTL and invalidates on Keto webhook; integrated into `fdb-app` use-cases.
- **G3 — Full RLS REST CRUD handlers in `RestCompiler`**: `handle_list`, `handle_insert`, `handle_update`, and `handle_delete`; parameterized SQL; filter operators `eq`, `neq`, `gt`, `gte`, `lt`, `lte`, `like`, `ilike`, `in`, `is`, `cs`, `cd`; Range pagination; safe column-name validation.
- **G4 — GraphQL hybrid**: pg_graphql passthrough for Query/Mutation under RLS plus async-graphql `Subscription` over `graphql-transport-ws` from `ChangeStreamSource`; introspection merges pg_graphql schema with subscription SDL. See [G4 Subscription Seam PR and PgRest LISTEN Follow-up Decisions](/g4-subscription-seam-pr-and-pgrest-listen-follow-up-decisions.md).
- **G5 — Subscription RLS enforcement**: each `EntityChange` from `fdb-realtime` must be re-queried as the subscriber with full `RlsContext` before delivery to prevent WAL bypass.
- **G6 — Gate tests**: `test_rest_select_with_eq_filter` across all 12 filter operators, `test_vault_dek_not_in_compiled_state`, `test_subscription_rls_drops_unauthorized_events`, and `test_keto_check_gates_mutation`.
- **G7 — `fdb-realtime` gRPC client**: `ChangeStreamSource` adapter for `flint-realtime-fabric` `WatchEntityType`; service-token auth; reconnect loop; fan-out to subscriber streams.

## Dependencies from Phase 2

- `CompiledState` and `DatabaseModel`: delivered in `p2-c003`.
- `RestCompiler` route registration: delivered in `p2-c004`; handler bodies are a Phase 3 deliverable.
- `StateManager` and `ArcSwap` hot reload: delivered in `p2-c005`.
- `fdb-auth` JWT verify to `RlsContext`: delivered in `p2-c001`.
- `SET LOCAL` RLS propagation: delivered in `p2-c002`.

## Pre-flight Check for G4

Before starting the GraphQL hybrid work, verify OQ-3 against the PG18 container:

```sql
SELECT extversion FROM pg_extension WHERE extname = 'pg_graphql';
```

If `pg_graphql` is not installed, defer G4 to `p3-c007` with a stub.

## Completed This Turn

T7 landed and PR #3 is ready for review, completing the `p3-c019` core gate. This follows the draft PR state captured in [p3-c019 PostgREST Query Engine Draft PR and T7 Follow-up](/p3-c019-postgrest-query-engine-draft-pr-and-t7-follow-up.md).

### T7: Route `fdb-reflection` Through `fdb-query`

Anti-drift refactor completed:

- `filters.rs` is now a thin bridge over `fdb-query`.
- `parse_filter_tree` now produces `FilterTree`.
- `render_where` produces the SQL `WHERE` fragment plus parameters.
- `bind_param` maps `QueryParam` to SQLx bindings for text, `text[]`, `jsonb`, and null.
- Retired duplicate `FilterOp`, `Filter`, `parse_filter`, and `build_where` implementations.
- Rewired `handle_list`, `handle_update`, and `handle_delete`.
- Preserved SQL shape per Base Rule #16:
  - `json_agg` wrapper remains intact.
  - `RETURNING row_to_json` remains intact.

### RFC-FORGE §3.3 / G6 Security Gate Port

`rest_filter_safety_gate.rs` was ported to the bridge. The gate still verifies:

- Exact per-operator SQL generation.
- Values are bound, not interpolated.
- Injection-style column names are rejected before SQL generation.
- Dangerous-looking values are safely bound as parameters.

## Validation

Passing checks:

- `fdb-query`: 69 tests.
- `fdb-postgres`: 4 tests.
- `fdb-reflection`: 46 unit tests.
- Integration gates: `5 + 2 + 2` passing.
- Four touched crates pass clippy pedantic with `-D warnings`.
- `cargo check --workspace` passes.

Known unrelated issue:

- Full-workspace clippy still trips a pre-existing lint in the `hello-component` example crate.
- The lint is from macro-generated WASI bindings: `used_underscore_items`.
- It is isolated to the example crate and not introduced by this change.
- The T9 checkbox was corrected to state the accurate per-crate clippy scope rather than overclaiming `--workspace` clippy success.

## Current Status

- `p3-c019` core tasks T1–T9 are complete.
- One authoritative PostgREST translator is now shared by both REST surfaces.
- `PgRest::execute` is live.
- Implemented core coverage includes:
  - 21 operators.
  - Logical trees.
  - Select.
  - Order.
  - Pagination.
  - Count.
  - Writes.
  - Safety checks.
- Three commits were committed and pushed.
- PR #3 is marked ready for review.

## Open PRs

- PR #2: G4 seam; review/merge pending.
- PR #3: `p3-c019` core; ready for review.

## Remaining Work: Parity Pass

The parity pass has not started. It is a large, decomposable body of work and may be run as a parallel workflow if requested.

Planned sequence:

1. **T10 — Resource embedding**
   - FK-join planner from `DatabaseModel` metadata.
   - Support `!fk`.
   - Support `!inner`.
   - Support spread.
   - Support nested embeddings.
2. **T11 — Full-text search variants**.
3. **T12 — Edge-case hardening**.

Each step should include an integration checkpoint.

# Citations

1. stdin
2. manual:Flint Forge/p3-auth-rls-keto