# Execution — p15-v1.0-production-readiness

**Phase:** p15-v1.0-production-readiness
**Backend:** native-tool
**Dispatch date:** 2026-07-08
**Driver:** Codex / kbd-execute

## Dispatch Contract

This phase is executed through native KBD progress tracking because there are no
`openspec/changes/p15-*` directories for the five p15 changes. The canonical
change inventory is `.kbd-orchestrator/phases/p15-v1.0-production-readiness/progress.json`.

Completed changes are validated by their implementation artifacts and the
phase gates in `goals.md`:

- p15-c001: pgrx/Postgres 18 extension stabilization.
- p15-c002: strict migration ordering and migration verification script.
- p15-c003: `forge-cli` operator commands and container packaging.
- p15-c004: CI database integration, k6 regression baselines, and DB pool metrics.
- p15-c005: README/security/roadmap updates, Helm chart, and Kiln cache hit path.

## Current Execution State

The implementation artifacts for all five changes are present. During final
state reconciliation, c004 and c005 still remained marked `in_progress` in
`progress.json`, while `current-waypoint.json` already reported 5/5 complete.
This execution pass reconciles the native KBD ledger after final targeted
verification.

## Verification Plan

- Run API version consistency check.
- Run Helm lint when `helm` is available.
- Run the canonical workspace CI script only if the wait budget and local time
  allow; otherwise record that final full CI remains the handoff action.

## Handoff

After c004/c005 are marked complete, the next lifecycle step is `/kbd-reflect`
for p15 closure and archive bookkeeping.
