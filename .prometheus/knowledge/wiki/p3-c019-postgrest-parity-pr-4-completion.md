---
type: Reference
id: p3-c019-postgrest-parity-pr-4-completion
title: 'p3-c019 PostgREST Parity PR #4 Completion'
tags:
- flint-forge
- postgrest
- fdb-query
- fdb-reflection
- auth-rls
- sql-safety
- phase-tracking
links:
- p3-auth-rls-keto-goals-and-compile-economy-build-config
- g4-subscription-seam-pr-and-pgrest-listen-follow-up-decisions
- p3-c019-core-complete-with-fdb-query-rest-filter-bridge
- p3-c019-postgrest-query-engine-draft-pr-and-t7-follow-up
sources:
- stdin
- manual:Flint Forge/p3-auth-rls-keto
- https://github.com/Know-Me-Tools/flint-forge/pull/4
timestamp: 2026-07-03T17:29:05.529031+00:00
created_at: 2026-07-03T17:29:05.529031+00:00
updated_at: 2026-07-03T17:29:05.529031+00:00
revision: 0
---

## Phase Context

- Project: Flint Forge
- Phase: `p3-auth-rls-keto`
- Status marker: `in_progress`
- Progress marker: `changes 7/9`
- Captured at: `2026-07-03T17:23:56Z`
- KBD root: `/Users/gqadonis/Projects/prometheus/flint-forge`

The phase gate remains the end-to-end auth/RLS/Keto/Cedar requirement described in [p3-auth-rls-keto Goals and Compile Economy Build Config](/p3-auth-rls-keto-goals-and-compile-economy-build-config.md): real `flint-gate` JWT propagation into Postgres RLS, Keto mutation gating, Cedar capability authorization, no plaintext credentials in logs or tracing spans, and parameterized SQL in CRUD handlers.

## Session Outcome

- The p3-c019 parity pass is complete: **T1–T13 shipped**.
- Opened PR #4: <https://github.com/Know-Me-Tools/flint-forge/pull/4>
- PR #2 remains open for the G4 seam, as tracked in [G4 Subscription Seam PR and PgRest LISTEN Follow-up Decisions](/g4-subscription-seam-pr-and-pgrest-listen-follow-up-decisions.md).
- PR #4 should be reviewed/merged before the next follow-up: wiring resource embedding into the `fdb-reflection` REST list handler.

## Delivered in `fdb-query`

`fdb-query` now has full PostgREST parity for T10–T13, extending the earlier p3-c019 query-engine work tracked in [p3-c019 Core Complete with fdb-query REST Filter Bridge](/p3-c019-core-complete-with-fdb-query-rest-filter-bridge.md) and [p3-c019 PostgREST Query Engine Draft PR and T7 Follow-up](/p3-c019-postgrest-query-engine-draft-pr-and-t7-follow-up.md).

### Resource embedding

- Added `EmbedSchema` while keeping `fdb-query` pure and avoiding layering inversion.
- Implemented correlated embedding using `json_agg` and `json_build_object` subselects.
- Supported PostgREST embedding forms:
  - `!fk`
  - `!inner`
  - top-level filtering by embedded resource via `EXISTS`
  - `...spread`
  - nested embedding

### Full-text search

- Implemented PostgREST FTS operators:
  - `fts`
  - `plfts`
  - `phfts`
  - `wfts`
- Operators map to the four PostgreSQL `*_tsquery` functions via `@@`.
- Supports configured search syntax such as `fts(english)`.
- Tsquery values are bound as SQL parameters.

### Edge-case coverage

Implemented and tested behavior for:

- empty `in`
- null handling
- composite primary keys
- reserved characters and Unicode
- `limit=0`
- large offsets
- order-by-embedded

## Security Defects Found and Fixed

The multi-agent verify phase found two real latent security defects; both were fixed with regression tests.

1. `UpdatePlan::render` and `DeletePlan::render` emitted relation and `SET` columns without validation.
   - Fix: relation and column identifiers are now validated before rendering.
2. `resolve_embeds` emitted caller-provided `parent_alias` and `parent_table` verbatim into correlation predicates.
   - Fix: these values are now validated at the embed-resolution gate.

## Verification Notes

The multi-agent workflow was used for advisory design, implementation, and adversarial review, but final authority was the actual working tree and compiler output.

Important verification findings:

- Agents had write access and mutated the working tree.
- One agent ran `git checkout`, reverting `lib.rs` mid-run.
- The final state was inspected from disk rather than trusted from agent summaries.
- Agents tested `fdb-query` in isolation and missed a cross-crate break:
  - adding `fts_config` to `Leaf` broke constructors in `fdb-postgres` and `fdb-reflection`.
  - `cargo check --workspace` caught the issue.
  - Both downstream crates were fixed.

Final validation completed:

- `fdb-query`: `128+29` tests passing
- `fdb-postgres`: `4` tests passing
- `fdb-reflection`: `46+` tests and gates passing
- `clippy -D warnings`: clean
- Workspace check: clean after fixing cross-crate `Leaf` constructor breakage

## Scope Boundary

The embedding engine is complete and tested, but it is not yet reachable over HTTP.

Remaining follow-up:

- Wire `DatabaseModel → EmbedSchema` into the `fdb-reflection` REST list handler.
- This makes the completed embedding engine available through the REST router.
- The follow-up is small and well-scoped and is flagged in the PR #4 test plan.

## PR Queue

- PR #2: G4 subscription seam, still awaiting review/merge.
- PR #4: PostgREST parity pass, awaiting review/merge.

# Citations

1. stdin
2. manual:Flint Forge/p3-auth-rls-keto
3. https://github.com/Know-Me-Tools/flint-forge/pull/4