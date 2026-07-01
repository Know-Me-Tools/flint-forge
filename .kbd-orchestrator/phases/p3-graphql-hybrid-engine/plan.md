# Phase Plan — p3-graphql-hybrid-engine

## Plan Status
`complete` — 9 changes ordered and fully specified

## Phase Summary

Phase 3 wires the GraphQL Hybrid Engine as described in CLAUDE.md §3.2:
- `POST /graphql` → pg_graphql passthrough under RLS (p3-c001)
- `GET /graphql` → WebSocket subscriptions via graphql-transport-ws (p3-c004)
- Subscription schema compiled from `DatabaseModel` by `GraphQlCompiler` (p3-c007)
- FRF `WatchEntityType` gRPC stream with Keto gate + per-event RLS re-query (p3-c002)
- `__schema` introspection merges pg_graphql ∪ subscription SDL (p3-c003)
- Extended GUC propagation: 3 additional SET LOCAL statements + `#[instrument]` (p3-c008)
- Keto tuple sync from FRF Iggy → `flint_meta.keto_tuples` (p3-c006)
- OQ-3 resolution: verify pg_graphql PG18 release (p3-c005)
- P2 opt-in: predicate pushdown for subscription throughput (p3-c009)

## Pre-Kickoff Gate

**OQ-3 must be verified before any coding begins:**
- Is a pg_graphql release compatible with Postgres 18 available?
- If yes: note the version and update Dockerfile. Proceed.
- If no: raise a blocker — p3-c001 depends on pg_graphql being available on PG18.

Run p3-c005 first to resolve this gate before touching any Rust code.

## Dependency Chain

```
p3-c005 (OQ-3 research) ──────────────────────────────► can start immediately
p3-c008 (GUC + instrument) ───────────────────────────► can start immediately

p3-c001 (pg_graphql passthrough) ─────────────────────► unblocked after p3-c005 verified
p3-c007 (GraphQlCompiler) ────────────────────────────► requires p3-c001 (RlsContext fields)
p3-c004 (WebSocket handler) ──────────────────────────► requires p3-c007 (subscription_schema)
p3-c002 (FabricChangeSource) ─────────────────────────► requires p3-c004 + OQ-FRF-1 resolved
p3-c003 (IntrospectionMerger) ────────────────────────► requires p3-c001 + p3-c007

p3-c006 (Keto sync) ─────────────────────────────────► requires OQ-8 resolution (parallel to above)
p3-c009 (predicate pushdown, P2) ────────────────────► requires p3-c002 fully complete + operator consent
```

## Execution Order

| Order | Change ID | What It Does | Blocked On | Priority |
|-------|-----------|-------------|------------|----------|
| 1 | `p3-c005-pg-graphql-pg18` | Verify pg_graphql PG18 release; close OQ-3 | Nothing | P0 |
| 2 | `p3-c008-extended-guc-propagation` | +3 SET LOCAL; `#[instrument]` on `verify_and_build` | Nothing | P0 |
| 3 | `p3-c001-graphql-passthrough` | POST /graphql + PgGraphQl::execute() | OQ-3 resolved | P0 |
| 4 | `p3-c007-graphql-compiler` | GraphQlCompiler + CompiledState.subscription_schema | p3-c001 | P0 |
| 5 | `p3-c004-graphql-transport-ws` | GET /graphql WebSocket + GraphQLSubscription | p3-c007 | P0 |
| 6 | `p3-c002-subscriptions` | FabricChangeSource + Keto gate + per-event RLS re-query | p3-c004 + OQ-FRF-1 | P0 |
| 7 | `p3-c003-introspection-merge` | IntrospectionMerger: pg_graphql ∪ subscription SDL | p3-c001 + p3-c007 | P0 |
| 8 | `p3-c006-keto-sync` | KetoSyncTask: FRF Iggy → flint_meta.keto_tuples | OQ-8 resolved | P0 |
| 9 | `p3-c009-predicate-pushdown` | Opt-in predicate pushdown (P2 deferral) | p3-c002 done | P2 |

**Changes 1 and 2 can run in parallel** (no shared files).  
**Change 7 can run in parallel with 6** (different crates, both need 1+7).  
**Change 9 is deferred** — begin only after p3-c002 is production-verified and operator has
accepted the data-leak risk in writing.

## Open Questions (must resolve before coding)

| ID | Question | Blocking | Resolution Path |
|----|----------|---------|----------------|
| OQ-3 | pg_graphql PG18 release available? | p3-c001 | p3-c005 (T1 research) |
| OQ-FRF-1 | WatchEntityType .proto location in flint-realtime-fabric? | p3-c002 | p3-c002 T0 (find .proto) |
| OQ-8 | FRF Iggy keto_changes topic schema? | p3-c006 | p3-c006 T0 (grep FRF source) |

## Non-Negotiable Security Invariants

These cannot be relaxed by any change in this phase:

1. **Per-event RLS re-query is MANDATORY** — p3-c002 implements it; p3-c009 does NOT remove it
2. **Never log JWT, claims, relation tuples, or keto_subject** — all `#[instrument]` spans skip these
3. **SET LOCAL inside transaction** — all 6 GUCs must be in one BEGIN block (p3-c008)
4. **Subscription schema reads CompiledState — never mutates it** (read-only Arc)
5. **Keto unavailable → deny subscription** (fail-closed, not fail-open)

## Recommended Agents

| Change | Recommended Subagent |
|--------|---------------------|
| p3-c005 | code-explorer (research + doc write) |
| p3-c008 | rust-reviewer after implementing |
| p3-c001 | rust-reviewer after implementing |
| p3-c007 | tdd-guide (write tests first for compiler output) |
| p3-c004 | rust-reviewer after implementing |
| p3-c002 | security-reviewer (mandatory — subscription RLS is critical path) |
| p3-c003 | tdd-guide (write test for merge behavior first) |
| p3-c006 | security-reviewer (Keto sync is security-critical) |
| p3-c009 | security-reviewer (operator risk doc review) |

## Phase Complete When

- All 8 P0 changes are `qa_passed` in `progress.json`
- `cargo test --workspace` passes
- `cargo clippy --workspace -- -D warnings` passes
- `GET /graphql` and `POST /graphql` both respond correctly in local integration
- p3-c009 may remain `pending` (P2 — deferred to Phase 5 or beyond)
