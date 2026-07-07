---
type: Reference
id: p3-auth-rls-keto-goals-and-compile-economy-build-config
title: p3-auth-rls-keto Goals and Compile Economy Build Config
tags:
- flint-forge
- auth-rls
- keto
- cedar-policy
- graphql-subscriptions
- rust-build
- compile-economy
links:
- integration-first-compile-economy
sources:
- stdin
- manual:Flint Forge/p3-auth-rls-keto
timestamp: 2026-07-03T14:17:26.590129+00:00
created_at: 2026-07-03T14:17:26.590129+00:00
updated_at: 2026-07-03T14:17:26.590129+00:00
revision: 0
---

## Phase Context

- Project: Flint Forge
- Phase: `p3-auth-rls-keto`
- Status: `in_progress`
- Progress marker: `changes 7/9`
- Captured at: `2026-07-03T14:04:53Z`
- KBD root: `/Users/gqadonis/Projects/prometheus/flint-forge`

## Phase Gate

All four authentication and authorization layers must be live end-to-end:

1. A real `flint-gate` JWT causes a real Postgres RLS row filter.
2. A Keto relation check gates mutations.
3. A Cedar policy controls capability-level access.
4. No plaintext credentials appear in any log line or tracing span.
5. CRUD handler bodies execute parameterized SQL.

## Goals

- **G1 — `forge-policy` Cedar evaluation**
  - Implement `PolicyEngine::evaluate(principal, action, resource, context)` returning allow/deny.
  - Load policy bundles from `flint_meta.cedar_policies`.
- **G2 — Keto relationship checks**
  - Enforce coarse Keto checks at subscribe-time and mutation-time.
  - `KetoCacheClient` caches relation tuples with TTL.
  - Cache invalidates on Keto webhook.
  - Integrate into `fdb-app` use-cases.
- **G3 — Full RLS REST CRUD handlers in `RestCompiler`**
  - Implement `handle_list`, `handle_insert`, `handle_update`, and `handle_delete`.
  - Use parameterized SQL.
  - Dispatch supported filter operators: `eq`, `neq`, `gt`, `gte`, `lt`, `lte`, `like`, `ilike`, `in`, `is`, `cs`, `cd`.
  - Support `Range` header pagination.
  - Validate column names for safety.
- **G4 — GraphQL hybrid**
  - Use `pg_graphql` passthrough for `Query`/`Mutation` under RLS.
  - Use `async-graphql` `Subscription` over `graphql-transport-ws`.
  - Subscription source pulls from `ChangeStreamSource`.
  - Introspection must merge `pg_graphql` schema with subscription SDL.
- **G5 — Subscription RLS enforcement**
  - For each `EntityChange` from `fdb-realtime`, re-query the changed row as the subscriber with full `RlsContext` before delivery.
  - This WAL-bypass protection is non-negotiable.
- **G6 — Gate tests**
  - `test_rest_select_with_eq_filter` covering all 12 filter operators.
  - `test_vault_dek_not_in_compiled_state` for DEK serde security.
  - `test_subscription_rls_drops_unauthorized_events`.
  - `test_keto_check_gates_mutation`.
- **G7 — `fdb-realtime` gRPC client**
  - Implement `ChangeStreamSource` adapter for `flint-realtime-fabric` `WatchEntityType` RPC.
  - Authenticate with service token.
  - Include reconnect loop.
  - Fan out to subscriber streams.

## Dependencies from Phase 2

Already delivered dependencies:

- `CompiledState` and `DatabaseModel` — `p2-c003`.
- `RestCompiler` route registration — `p2-c004`; handler bodies remain a Phase 3 deliverable.
- `StateManager` + `ArcSwap` hot reload — `p2-c005`.
- `fdb-auth` JWT verification to `RlsContext` — `p2-c001`.
- `SET LOCAL` RLS propagation — `p2-c002`.

## GraphQL Pre-flight Check

Before starting G4, verify whether `pg_graphql` is installed in the PG18 container:

```sql
SELECT extversion
FROM pg_extension
WHERE extname = 'pg_graphql';
```

If `pg_graphql` is not installed, defer G4 to `p3-c007` with a stub.

## Build Configuration Changes

The development compile-economy policy was wired into the actual build files, aligning with [Integration-First Delivery and Compile Economy](/integration-first-compile-economy.md).

| File | Change |
|---|---|
| `Cargo.toml` | Added `[profile.dev]` with `opt-level = 0` and `debug = "line-tables-only"`; added `[profile.dev.package."*"]` with `opt-level = 1`; added explicit `[profile.release]`. |
| `.cargo/config.toml.example` | Added fast-linker template with verified local linker path: `/opt/homebrew/bin/ld64.lld`. Previous documentation had assumed an LLVM keg path, which did not match this Homebrew installation. |
| `.gitignore` | Added ignore rule for machine-specific live `.cargo/config.toml`. |
| `docs/RUST-DEVELOPMENT-MANAGEMENT.md` | Corrected the aarch64 linker path and example-file note to match observed local state. |

The live `.cargo/config.toml` remains intentionally untracked because it is machine-specific.

## Validation

`cargo check --workspace` completed cleanly after the build-profile changes:

```text
Finished dev profile ... in 1m 26s
```

No check errors were reported across the workspace, including `fdb-gateway`, `fdb-app`, and `forge-policy`.

## Git Status and Next Work

- Changes were not committed; repository workflow requires committing only on explicit request.
- Candidate commit grouping when approved:
  - `chore:` for build configuration.
  - `docs:` for development-management documentation updates.
- Main remaining implementation gap: G4 GraphQL hybrid subscription over `graphql-transport-ws`.
- Alternative next step: commit development-management docs plus build config before resuming `p3-auth-rls-keto` implementation.

# Citations

1. stdin
2. manual:Flint Forge/p3-auth-rls-keto