# Plan Handoff — p3.5-ci-postgres-hardening

**From:** kbd-plan (claude-code)
**Date:** 2026-07-03
**Phase:** p3.5-ci-postgres-hardening

## Summary

**5 OpenSpec changes**, ordered so the CI gate is trustworthy before DB stages are
added to it: **c001 (unblock red clippy gate) → c002 (green existing gateway tests) →
c003 (add PG service + `cargo test` to CI) → c004 (make DB tests gating + add embedding/
PgRest coverage) → c005 (reconcile p3 bookkeeping)**.

## Ordering rationale

CI is currently *red* (c001/G4) and runs *no tests* (c003/G1) — so the pipeline is
untrustworthy today. Fix the gate, green the non-DB tests, then add the DB service, then
make the integration tests consume it. c005 is docs/state-only and sequenced last so it
captures the final delivered set.

## First change to apply

`p35-c001-clippy-unblock-hello-component` — a narrow `#[allow(clippy::used_underscore_items)]`
on hello-component's generated WASI bindings. Mechanical; unblocks CI validation for
everything after it.

## Notes / open questions carried to execute

- **OQ-9** (in c003): prebuilt PG18+pgvector+pg_graphql image vs. build a pinned
  Dockerfile — decide in c003's design.
- Recommended DB provisioning: extend the **Dagger** pipeline (service binding), not
  GitHub Actions, to keep local == CI.
- `progress.json` change_order set (5 changes, all `pending`); waypoint `next_action`
  → `/kbd-apply p35-c001-...`.

_plan:before/after hooks + stage-gate not fired — KBD shell libs unresolvable in this
env (KBD_ORCHESTRATOR_ROOT unset); surfaced rather than silently skipped._
