---
type: Reference
id: p3-auth-rls-keto-session-status-and-pr-review-queue
title: p3-auth-rls-keto Session Status and PR Review Queue
tags:
- flint-forge
- auth-rls
- keto
- cedar-policy
- postgrest
- graphql-subscriptions
- phase-tracking
links:
- p3-auth-rls-keto-goals-and-compile-economy-build-config
- p3-c019-postgrest-query-engine-draft-pr-and-t7-follow-up
- p3-c019-core-complete-with-fdb-query-rest-filter-bridge
- g4-subscription-seam-pr-and-pgrest-listen-follow-up-decisions
sources:
- stdin
- manual:Flint Forge/p3-auth-rls-keto
timestamp: 2026-07-03T16:26:53.962343+00:00
created_at: 2026-07-03T16:26:53.962343+00:00
updated_at: 2026-07-03T16:26:53.962343+00:00
revision: 0
---

## Phase Context

- Project: Flint Forge
- Phase: `p3-auth-rls-keto`
- Status: `in_progress`
- Progress marker: `changes 7/9`
- Captured at: `2026-07-03T16:22:57Z`
- KBD root: `/Users/gqadonis/Projects/prometheus/flint-forge`

The phase gate remains the end-to-end auth/RLS/Keto/Cedar requirement described in [p3-auth-rls-keto Goals and Compile Economy Build Config](/p3-auth-rls-keto-goals-and-compile-economy-build-config.md): real `flint-gate` JWT propagation into Postgres RLS, Keto relation checks gating mutations, Cedar capability-level policy enforcement, no plaintext credentials in logs or tracing spans, and parameterized SQL in CRUD handlers.

## Phase Goals

- **G1 — `forge-policy` Cedar policy evaluation**
  - `PolicyEngine::evaluate(principal, action, resource, context)` returns allow/deny.
  - Policy bundles load from `flint_meta.cedar_policies`.
- **G2 — Keto coarse relationship checks**
  - Checks run at subscribe-time and mutation-time.
  - `KetoCacheClient` caches relation tuples with TTL.
  - Cache invalidates on Keto webhook.
  - Integrated into `fdb-app` use-cases.
- **G3 — Full RLS CRUD handler bodies in `RestCompiler`**
  - Implement `handle_list`, `handle_insert`, `handle_update`, and `handle_delete`.
  - Use parameterized SQL.
  - Support filter operator dispatch: `eq`, `neq`, `gt`, `gte`, `lt`, `lte`, `like`, `ilike`, `in`, `is`, `cs`, `cd`.
  - Support `Range` header pagination.
  - Validate column-name safety.
- **G4 — GraphQL hybrid**
  - `pg_graphql` passthrough for `Query`/`Mutation` under RLS.
  - `async-graphql` `Subscription` over `graphql-transport-ws` pulling from `ChangeStreamSource`.
  - Introspection merges `pg_graphql` schema with subscription SDL.
- **G5 — Subscription RLS enforcement**
  - For each `EntityChange` from `fdb-realtime`, re-query the changed row as the subscriber with full `RlsContext` before delivery.
  - This WAL-bypass protection is non-negotiable.
- **G6 — Gate tests**
  - `test_rest_select_with_eq_filter` and all 12 filter operators.
  - `test_vault_dek_not_in_compiled_state` for DEK serde security.
  - `test_subscription_rls_drops_unauthorized_events`.
  - `test_keto_check_gates_mutation`.
- **G7 — `fdb-realtime` gRPC client**
  - `ChangeStreamSource` adapter connects to `flint-realtime-fabric` `WatchEntityType` RPC.
  - Authenticated via service token.
  - Includes reconnect loop and fan-out to subscriber streams.

## Dependencies from Phase 2

- `CompiledState` and `DatabaseModel`: delivered in `p2-c003`.
- `RestCompiler` route registration: delivered in `p2-c004`; handler bodies are Phase 3 deliverables.
- `StateManager` plus `ArcSwap` hot-reload: delivered in `p2-c005`.
- `fdb-auth` JWT verification to `RlsContext`: delivered in `p2-c001`.
- `SET LOCAL` RLS propagation: delivered in `p2-c002`.

## Pre-flight Requirement for G4

Before starting the GraphQL hybrid work, verify OQ-3 against the PG18 container:

```sql
SELECT extversion FROM pg_extension WHERE extname = 'pg_graphql';
```

If `pg_graphql` is not installed, defer G4 to `p3-c007` with a stub.

## Current PR State

- PR #3: <https://github.com/Know-Me-Tools/flint-forge/pull/3>
  - `p3-c019` PostgREST query engine.
  - Status: ready for review.
  - This is the continuation of the PostgREST work tracked in [p3-c019 PostgREST Query Engine Draft PR and T7 Follow-up](/p3-c019-postgrest-query-engine-draft-pr-and-t7-follow-up.md) and [p3-c019 Core Complete with fdb-query REST Filter Bridge](/p3-c019-core-complete-with-fdb-query-rest-filter-bridge.md).
- PR #2: <https://github.com/Know-Me-Tools/flint-forge/pull/2>
  - G4 GraphQL subscription seam.
  - Status: open and awaiting review/merge.
  - Related decisions and validation are tracked in [G4 Subscription Seam PR and PgRest LISTEN Follow-up Decisions](/g4-subscription-seam-pr-and-pgrest-listen-follow-up-decisions.md).

## Next Work

Begin the `p3-c019` parity pass:

1. T10: resource embedding.
2. T11: full-text search.
3. T12: edge cases.

Execution mode decision remains open: run single-threaded or use a workflow to fan out in parallel.

# Citations

1. [1] stdin
2. [2] manual:Flint Forge/p3-auth-rls-keto