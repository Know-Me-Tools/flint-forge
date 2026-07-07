---
type: Reference
id: g4-subscription-seam-pr-and-pgrest-listen-follow-up-decisions
title: G4 Subscription Seam PR and PgRest LISTEN Follow-up Decisions
tags:
- flint-forge
- auth-rls
- graphql-subscriptions
- pgrest
- postgres-listen
- keto
- phase-tracking
links:
- p3-auth-rls-keto-goals-and-compile-economy-build-config
sources:
- stdin
- manual:Flint Forge/p3-auth-rls-keto
timestamp: 2026-07-03T15:38:09.895603+00:00
created_at: 2026-07-03T15:38:09.895603+00:00
updated_at: 2026-07-03T15:38:09.895603+00:00
revision: 0
---

## Phase Context

- Project: Flint Forge
- Phase: `p3-auth-rls-keto`
- Status: `in_progress`
- Progress marker: `changes 7/9`
- Captured at: `2026-07-03T15:02:29Z`
- KBD root: `/Users/gqadonis/Projects/prometheus/flint-forge`

The phase gate remains the end-to-end auth/RLS/Keto/Cedar requirement described in [p3-auth-rls-keto Goals and Compile Economy Build Config](/p3-auth-rls-keto-goals-and-compile-economy-build-config.md): JWT-driven Postgres RLS, Keto mutation gating, Cedar capability policy, no plaintext credentials in logs/spans, and parameterized CRUD SQL.

## Completed This Turn

- Opened PR #2: <https://github.com/Know-Me-Tools/flint-forge/pull/2>
  - Implements the G4 GraphQL subscription wiring seam.
  - Branched from `main`.
  - Staged the coherent G4 set:
    - 4 subscription seam files.
    - `p3-c016` `fdb-app` work that G4 builds on.
    - `gate_tests.rs`.
    - `Cargo.lock`.
    - `p3-c016` archive move.
  - Excluded KBD state and tool-generated wiki pages from the PR.
- Validation before commit:
  - `cargo test -p fdb-app`
  - Result: all tests passed, including:
    - `test_subscription_rls_drops_unauthorized_events`
    - Keto gate tests
    - meta-listener tests
  - Test-wait usage: `2/3`.

## Remaining Follow-ups

Two remaining follow-ups are coupled:

1. A Postgres `LISTEN`-based in-process `ChangeStreamSource` produces change events.
2. `PgRest::execute` performs the RLS re-query that filters subscription events before delivery.

The coupling is important because subscription RLS enforcement requires re-querying each changed row as the subscriber with a full `RlsContext` before delivery. WAL or notification events must not bypass RLS.

## `PgRest::execute` Design Fork

The 12-operator query builder already exists at:

```text
crates/fdb-reflection/src/compilers/filters.rs::build_where
```

Current properties:

- Used by REST mutation handlers.
- Operates on `sqlx::PgPool` inside `fdb-reflection`.
- Supports the REST filter operator surface:
  - `eq`
  - `neq`
  - `gt`
  - `gte`
  - `lt`
  - `lte`
  - `like`
  - `ilike`
  - `in`
  - `is`
  - `cs`
  - `cd`

`PgRest` is separate from that implementation:

- Lives in `fdb-postgres`.
- Uses a deadpool-based adapter rather than `sqlx::PgPool`.

Implementation options for `PgRest::execute`:

### b1 — Subscription-only query shape

Recommended option.

- Implement only the subscription RLS re-query shape.
- Expected shape:
  - primary-key/equality filters only (`eq`)
  - `LIMIT 1`
  - matches what `build_pk_filters` emits
- Benefits:
  - Small scope.
  - YAGNI-aligned.
  - Unblocks subscriptions without duplicating the full REST builder.

### b2 — Full PostgREST builder in `fdb-postgres`

- Implement the complete 12-operator builder in `fdb-postgres`.
- Downside: duplicates `build_where` behavior already present in `fdb-reflection`.

### b3 — Extract `build_where` into a shared crate

- Move reusable filter SQL construction into a shared crate consumed by both `fdb-reflection` and `fdb-postgres`.
- Downside: larger refactor and broader integration risk.

## Postgres LISTEN ChangeStreamSource Follow-up

The in-process Postgres `LISTEN` adapter is cleanly separable as a new `ChangeStreamSource`, but requires a database migration:

- Add triggers for RLS-enabled tables.
- Triggers emit `NOTIFY` on DML.
- The listener source consumes notifications and feeds subscriber streams.

Open architecture decision:

- Replace `FabricChangeSource`; or
- Add the LISTEN source behind an environment variable or feature flag while keeping `FabricChangeSource`/FRF as the default when `flint-realtime-fabric` lands.

## Requested Decisions

Before implementation continues, choose:

1. `PgRest::execute` scope:
   - `b1`: subscription-only, recommended
   - `b2`: full PostgREST builder duplicated in `fdb-postgres`
   - `b3`: extract shared query builder crate
2. LISTEN source rollout:
   - replace `FabricChangeSource`; or
   - add LISTEN behind an env/feature flag and keep FRF as the default future path

## Planned Next Work After Decisions

- Implement `PgRest::execute`, preferably option `b1`.
- Implement the Postgres `LISTEN` `ChangeStreamSource`.
- Add the `NOTIFY` trigger migration.
- Wire the LISTEN source into the gateway factory.
- Run final validation using remaining test-wait budget:
  - `cargo check`
  - `cargo test`
  - test-wait usage target: `3/3`

## Scratchpad

Detailed follow-up implementation plan saved in:

```text
g4-followups-plan.md
```

# Citations

1. stdin
2. manual:Flint Forge/p3-auth-rls-keto