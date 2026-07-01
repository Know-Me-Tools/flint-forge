# Goals — p3-auth-rls-keto

**Phase gate:** All four authentication and authorization layers are live end-to-end: a real flint-gate JWT causes a real Postgres RLS row filter, a Keto relation check gates mutations, and a Cedar policy controls capability-level access. Zero plaintext credentials in any log line or tracing span. CRUD handler bodies execute parameterized SQL.

---

## Goals

- **G1** — `forge-policy`: Cedar policy evaluation crate — `PolicyEngine::evaluate(principal, action, resource, context)` returns allow/deny; policy bundles loaded from `flint_meta.cedar_policies` table
- **G2** — Keto coarse relationship check at subscribe-time and mutation-time — `KetoCacheClient` caches relation tuples with TTL, invalidated on Keto webhook; integrated into `fdb-app` use-cases
- **G3** — Full RLS CRUD handler bodies in `RestCompiler` — `handle_list`, `handle_insert`, `handle_update`, `handle_delete` with parameterized SQL, filter operator dispatch (eq/neq/gt/gte/lt/lte/like/ilike/in/is/cs/cd), Range header pagination, column-name safety validation
- **G4** — GraphQL hybrid: pg_graphql passthrough for Query/Mutation under RLS + async-graphql `Subscription` over `graphql-transport-ws` pulling from `ChangeStreamSource`; introspection merges pg_graphql schema ∪ subscription SDL
- **G5** — Subscription RLS enforcement: for each `EntityChange` from `fdb-realtime`, re-query the changed row as the subscriber with full `RlsContext` before delivering (WAL-bypass protection — non-negotiable)
- **G6** — Gate tests: `test_rest_select_with_eq_filter` (all 12 filter operators), `test_vault_dek_not_in_compiled_state` (DEK serde security gate), `test_subscription_rls_drops_unauthorized_events`, `test_keto_check_gates_mutation`
- **G7** — `fdb-realtime` gRPC client: `ChangeStreamSource` adapter connecting to `flint-realtime-fabric` `WatchEntityType` RPC; authenticated via service token, reconnect loop, fan-out to subscriber streams

---

## Dependencies from Phase 2

- `CompiledState` and `DatabaseModel` — delivered (p2-c003)
- `RestCompiler` route registration — delivered (p2-c004); handler bodies are the Phase 3 deliverable
- `StateManager` + `ArcSwap` hot-reload — delivered (p2-c005)
- `fdb-auth` JWT verify → `RlsContext` — delivered (p2-c001)
- `SET LOCAL` RLS propagation — delivered (p2-c002)

## Pre-flight check

Before starting GraphQL hybrid (G4), verify OQ-3: `SELECT extversion FROM pg_extension WHERE extname = 'pg_graphql';` against PG18 container. If pg_graphql is not installed, defer G4 to p3-c007 with a stub.
