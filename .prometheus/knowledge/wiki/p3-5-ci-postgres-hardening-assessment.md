---
type: Reference
id: p3-5-ci-postgres-hardening-assessment
title: p3.5 CI Postgres Hardening Assessment
tags:
- flint-forge
- ci
- postgres
- pgvector
- pg-graphql
- clippy
- phase-tracking
links:
- executor-session-completion-p3-5-ci-postgres-hardening
- executor-progress-record-p3-5-ci-postgres-hardening-0-of-0
- p3-auth-rls-keto-goals-and-compile-economy-build-config
sources:
- stdin
timestamp: 2026-07-03T20:37:52.758786+00:00
created_at: 2026-07-03T20:37:52.758786+00:00
updated_at: 2026-07-03T20:37:52.758786+00:00
revision: 0
---

## Phase Context

- Project: Flint Forge
- Phase: `p3.5-ci-postgres-hardening`
- KBD root: `/Users/gqadonis/Projects/prometheus/flint-forge`
- Captured: `2026-07-03T20:36:06Z`
- Source marker: `manual:Flint Forge/p3.5-ci-postgres-hardening`
- Assessment commit: `e8008f7` pushed

This assessment corroborates earlier minimal phase-tracking records for [Executor Session Completion: p3.5-ci-postgres-hardening](/executor-session-completion-p3-5-ci-postgres-hardening.md) and [Executor Progress Record: p3.5-ci-postgres-hardening 0 of 0](/executor-progress-record-p3-5-ci-postgres-hardening-0-of-0.md) with concrete findings and next actions.

## Phase Gate

The manually proven real-time and REST Postgres paths from `p3` must become CI-gating. The pre-existing `fdb-gateway` test debt must also be cleared so `cargo test --workspace` is green and meaningful with a database in CI.

This phase is seeded from `p3-auth-rls-keto` reflection and handoff work. It closes the gap where live-Postgres behavior was proven only through manual `--ignored` test runs. The previous auth/RLS/Keto phase context is related to [p3-auth-rls-keto Goals and Compile Economy Build Config](/p3-auth-rls-keto-goals-and-compile-economy-build-config.md).

## Goals

- **G1 — CI Postgres service**
  - Provision a PG18 database with `pgvector` and `pg_graphql` in CI.
  - Wire this through `scripts/ci-check.sh` and/or Dagger.
  - Export `DATABASE_URL` so DB-backed tests run in CI.
  - Resolves `OQ-9`.
- **G2 — Un-ignore live-PG tests**
  - Remove `#[ignore]` or gate on `DATABASE_URL` presence rather than the ignore attribute.
  - Bring these DB-gated tests into CI coverage:
    - `fdb-realtime/tests/listen_live_pg.rs`
    - `fdb-reflection` pgvector tests
    - `fdb-reflection` meta-listener tests
    - two a2ui tests
  - Add DB-backed coverage for:
    - embedding REST path: `select=*,child(*)` must produce correct nested JSON
    - `PgRest::execute`
- **G3 — Fix pre-existing `fdb-gateway` test debt**
  - Isolate `keto_sync_config_ignores_non_numeric_env` so parallel `set_var` usage no longer flakes.
  - Clear the `uninlined_format_args` lint in `tests/a2ui_seed_test.rs`.
- **G4 — Workspace clippy gate clean end-to-end**
  - `cargo clippy --workspace --all-targets -- -D warnings` must pass.
  - Current blocker: the `hello-component` example crate emits macro-generated `used_underscore_items` lint failures from WASI bindings.
  - Required fix: allow or annotate the generated-code lint narrowly.
- **G5 — Reconcile p3 phase bookkeeping**
  - Record `c019` PostgREST engine as delivered.
  - Record `c020` LISTEN source as delivered.
  - Mark `c017` superseded by `c020`.
  - Resolve or re-scope `c018` against the merged introspection work.

## Assessment Findings

Headline: current CI verifies almost nothing relevant to the phase gate.

| Goal | Status | Finding |
|---|---|---|
| G1 — CI Postgres service | Not met | `ci-check.sh` runs fmt, clippy, and `cargo check` only. It does not run `cargo test` and does not provision a database. Dagger uses `rust:1.90-bookworm` with no Postgres sidecar. |
| G2 — live-PG tests running | Not met | At least five DB-gated test files are dark in CI; no DB-backed embedding or `PgRest::execute` test exists. |
| G3 — gateway test debt | Not met | `keto_sync` env-var test can flake under parallel `set_var`; `a2ui_seed_test` still has `uninlined_format_args` lint debt. |
| G4 — workspace clippy clean | Not met; blocks CI now | `cargo clippy --workspace --all-targets` is already red on `hello-component` generated WASI bindings. |
| G5 — p3 bookkeeping | Not met | `c019` and `c020` are merged but untracked; `c017` should be marked superseded; `c018` overlaps merged introspection work. |

## Recommended Execution Order

1. **G4** — unblock the currently failing workspace clippy gate.
2. **G3** — remove gateway flake/lint debt before expanding CI scope.
3. **G1** — provision CI Postgres with required extensions.
4. **G2** — un-ignore/gate live-PG tests and add missing DB-backed REST coverage.
5. **G5** — reconcile phase bookkeeping.

Rationale: CI is already failing before DB provisioning work begins. Fixing clippy first prevents later database work from being masked by unrelated red gates.

## Open Decision: DB Provisioning Mechanism

The assessment recommends extending the existing **Dagger pipeline with a Postgres service binding** rather than adding GitHub Actions `services:` directly.

Rationale:

- Keeps CI behavior runnable locally and in CI.
- Centralizes the service dependency in the existing pipeline abstraction.
- Avoids divergent local-vs-GitHub database setup.

Unresolved: whether to use a prebuilt PG18 image containing both `pgvector` and `pg_graphql`, or build a project-specific image.

## Artifacts Written

- `assessment.md`
- `handoffs/assess.md`
- `assessment_complete: true`

## Operational Caveat

Assessment hooks and the stage-gate were not fired because the KBD shell libraries were not resolvable through `KBD_ORCHESTRATOR_ROOT` in the execution environment. Durable artifacts and the progress flag were written; only hook side effects were skipped.

## Next Step

Run `/kbd-plan p3.5-ci-postgres-hardening` to convert the assessment into an ordered change list using the recommended order: `G4 → G3 → G1 → G2 → G5`.

# Citations

1. stdin