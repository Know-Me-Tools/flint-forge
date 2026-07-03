# Assess Handoff — p3.5-ci-postgres-hardening

**From:** kbd-assess (claude-code)
**Date:** 2026-07-03
**Phase:** p3.5-ci-postgres-hardening

## Key gaps found

- **G1/G2**: `scripts/ci-check.sh` runs fmt + clippy + `cargo check` only — **no
  `cargo test`, no Postgres, no `DATABASE_URL`**. The Dagger pipeline
  (`rust:1.90-bookworm`, no PG sidecar) inherits this. So ≥5 DB-gated integration test
  files are dark in CI, and there is no DB-backed test of the embedding REST path or
  `PgRest::execute`.
- **G4**: the current CI clippy gate (`cargo clippy --workspace --all-targets`) is
  **already red** on `examples/hello-component` (`used_underscore_items` from generated
  WASI bindings) — unblock this first, it's cheap.
- **G3**: `keto_sync` env-var test flakes under parallel `set_var`; `a2ui_seed_test.rs`
  trips `uninlined_format_args`.
- **G5**: p3 `progress.json` bookkeeping unreconciled — c019/c020 merged but untracked;
  c017 superseded-by-c020; c018 overlaps merged work.

## Open questions for Plan

- OQ-9: DB provisioning mechanism (Dagger service binding recommended, to keep
  local == CI) and whether a PG18 image with both `pgvector ≥ 0.7` and `pg_graphql`
  exists prebuilt or must be built.
- One `cargo test` vs. a separate DB test stage.

## Suggested plan order

G4 (unblock CI) → G3 (green existing tests) → G1 (DB service) → G2 (un-ignore + new
DB tests) → G5 (bookkeeping).

_Note: assess:before/after hooks + stage-gate not fired — KBD shell libs
(`hooks.sh`, `stage-gate.sh`) unresolvable in this env (KBD_ORCHESTRATOR_ROOT unset);
surfaced rather than silently skipped. Assessment is the first stage, so its gate
passes trivially._
