# Reflect Handoff — p3-auth-rls-keto

**From:** kbd-reflect (claude-code)
**Date:** 2026-07-03
**Phase:** p3-auth-rls-keto

## Summary

Goals G1–G6 MET (G3 exceeded to full PostgREST parity via the new `fdb-query`
crate; G4 subscription seam wired; G5 RLS re-query live). G7 PARTIAL: the FRF
`WatchEntityType` path is still OQ-FRF-1-blocked, but the capability is delivered
via the in-process `ListenChangeSource` (c020) — env-selectable, migration 0006,
live-PG tested. All session work (G4 seam, c019, c020) is merged to `main`.

## Deltas found

- `progress.json` drifted: it tracks c010–c018 (7/9, c017/c018 `pending`) but the
  delivered work landed as un-tracked c019 + c020 + the G4 seam. c017 is superseded
  by c020; c018 overlaps already-merged introspection work.
- Cross-crate break risk confirmed real (FTS `fts_config` field) — only
  `cargo check --workspace` caught it, not per-crate tests.
- Live-PG tests are `#[ignore]`-gated (no CI Postgres yet).

## Corrective actions (next)

1. Reconcile `progress.json`: register c019/c020, mark c017 superseded-by-c020,
   resolve/re-scope c018 — then the phase can close honestly.
2. Add a CI Postgres service to make the live-PG tests gating.

## Recommended next phase

Housekeeping reconcile first, then either OQ-FRF-1 resolution + CI-PG hardening,
or advance to Flint Kiln (`fke-*`) WASM edge-function gateway per the roadmap.

_Note: reflection.md written. Hook/stage-gate shell libs (`waypoint.sh`,
`stage-gate.sh`) were not resolvable in this environment (KBD_ORCHESTRATOR_ROOT
unset), so `reflect:after`/`phase:after` hooks were not fired programmatically and
the waypoint was not auto-advanced — surfaced here rather than silently skipped._
