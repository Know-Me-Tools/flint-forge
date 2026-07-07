---
type: Reference
id: p3-c019-postgrest-query-engine-draft-pr-and-t7-follow-up
title: p3-c019 PostgREST Query Engine Draft PR and T7 Follow-up
tags:
- flint-forge
- auth-rls
- keto
- postgrest
- fdb-query
- graphql-subscriptions
- phase-tracking
links:
- p3-auth-rls-keto-goals-and-compile-economy-build-config
- g4-subscription-seam-pr-and-pgrest-listen-follow-up-decisions
sources:
- stdin
- manual:Flint Forge/p3-auth-rls-keto
timestamp: 2026-07-03T16:18:10.736832+00:00
created_at: 2026-07-03T16:18:10.736832+00:00
updated_at: 2026-07-03T16:18:10.736832+00:00
revision: 0
---

## Phase Context

- Project: Flint Forge
- Phase: `p3-auth-rls-keto`
- Status: `in_progress`
- Progress marker: `changes 7/9`
- Captured at: `2026-07-03T16:05:48Z`
- KBD root: `/Users/gqadonis/Projects/prometheus/flint-forge`

The phase gate remains the end-to-end auth/RLS/Keto/Cedar requirement described in [p3-auth-rls-keto Goals and Compile Economy Build Config](/p3-auth-rls-keto-goals-and-compile-economy-build-config.md): real `flint-gate` JWT propagation into Postgres RLS, Keto mutation gating, Cedar capability authorization, no plaintext credentials in logs or tracing spans, and parameterized SQL in CRUD handlers.

## Phase Goals

- **G1 — `forge-policy` Cedar evaluation**
  - `PolicyEngine::evaluate(principal, action, resource, context)` returns allow/deny.
  - Policy bundles load from `flint_meta.cedar_policies`.
- **G2 — Keto relationship checks**
  - Coarse relationship checks run at subscribe-time and mutation-time.
  - `KetoCacheClient` caches relation tuples with TTL.
  - Cache invalidates on Keto webhook.
  - Integrated into `fdb-app` use-cases.
- **G3 — Full RLS REST CRUD handlers in `RestCompiler`**
  - Implement `handle_list`, `handle_insert`, `handle_update`, `handle_delete`.
  - Use parameterized SQL.
  - Dispatch supported filter operators: `eq`, `neq`, `gt`, `gte`, `lt`, `lte`, `like`, `ilike`, `in`, `is`, `cs`, `cd`.
  - Support `Range` header pagination.
  - Validate column names for safety.
- **G4 — GraphQL hybrid**
  - `pg_graphql` passthrough for Query/Mutation under RLS.
  - `async-graphql` `Subscription` over `graphql-transport-ws` fed by `ChangeStreamSource`.
  - Introspection merges `pg_graphql` schema with subscription SDL.
  - PR #2 covers the G4 subscription seam; see [G4 Subscription Seam PR and PgRest LISTEN Follow-up Decisions](/g4-subscription-seam-pr-and-pgrest-listen-follow-up-decisions.md).
- **G5 — Subscription RLS enforcement**
  - For every `EntityChange` from `fdb-realtime`, re-query the changed row as the subscriber with full `RlsContext` before delivery.
  - WAL-bypass protection is non-negotiable.
- **G6 — Gate tests**
  - `test_rest_select_with_eq_filter` covering all 12 filter operators.
  - `test_vault_dek_not_in_compiled_state` for DEK serde security.
  - `test_subscription_rls_drops_unauthorized_events`.
  - `test_keto_check_gates_mutation`.
- **G7 — `fdb-realtime` gRPC client**
  - `ChangeStreamSource` adapter connects to `flint-realtime-fabric` `WatchEntityType` RPC.
  - Authenticated with service token.
  - Includes reconnect loop and fan-out to subscriber streams.

## Phase 2 Dependencies Already Delivered

- `CompiledState` and `DatabaseModel` from `p2-c003`.
- `RestCompiler` route registration from `p2-c004`; Phase 3 owns handler bodies.
- `StateManager` and `ArcSwap` hot reload from `p2-c005`.
- `fdb-auth` JWT verification into `RlsContext` from `p2-c001`.
- `SET LOCAL` RLS propagation from `p2-c002`.

## Pre-flight Requirement for GraphQL Hybrid

Before starting or completing G4, verify whether `pg_graphql` is present in the PG18 container:

```sql
SELECT extversion FROM pg_extension WHERE extname = 'pg_graphql';
```

If `pg_graphql` is not installed, defer G4 to `p3-c007` with a stub.

## Current Session Result

- Branch pushed: `feat/p3-c019-postgrest-query-engine` to `origin`.
- Draft PR opened: <https://github.com/Know-Me-Tools/flint-forge/pull/3>
  - PR #3 is intentionally marked **draft** because T7 and the parity pass remain.
  - Includes both existing commits: foundation plus read/write core and `PgRest::execute` wiring.
- No new commit was needed in this session; the `fdb-query` engine and `PgRest::execute` work had already been committed and verified clean.
- Validation already reported clean:
  - 73 tests passing.
  - `clippy -D warnings` clean.
  - workspace check clean.

## Open Pull Requests

- **PR #3** — draft, `p3-c019` PostgREST engine core: <https://github.com/Know-Me-Tools/flint-forge/pull/3>
- **PR #2** — G4 GraphQL subscription seam, ready for review/merge: <https://github.com/Know-Me-Tools/flint-forge/pull/2>
- **PR #1** — already merged.

## Next Work

1. Implement **T7**:
   - Route `fdb-reflection` REST handlers through `fdb-query`.
   - Retire duplicate `filters::build_where`.
   - Keep reflection REST tests green.
2. Run the parity pass:
   - Resource embedding.
   - Full-text search.
   - Edge cases.
3. Mark PR #3 ready for review after T7 lands.
4. Review/merge PR #2 separately.

# Citations

1. stdin
2. manual:Flint Forge/p3-auth-rls-keto