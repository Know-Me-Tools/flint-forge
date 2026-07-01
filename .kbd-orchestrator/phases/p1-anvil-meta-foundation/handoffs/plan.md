# Stage Handoff: plan → execute

**Stage:** plan  
**Written:** 2026-06-30  
**For:** /kbd-execute p1-anvil-meta-foundation

## Summary

11 changes planned across 4 dependency batches. p1-c005 (JWT contract pin) is already complete. The critical path runs through p1-c007 (new ext-flint-meta crate) — all meta build-up changes (c008, c009, c010, c011) wait on it. Two open questions must be resolved at execution time: OQ-9 (hooks Cargo.toml pgrx version, before p1-c002) and OQ-10 (Dockerfile current state, before p1-c004).

## Artifacts Written This Stage

- `.kbd-orchestrator/phases/p1-anvil-meta-foundation/plan.md` — ordered change list, batch structure, dependency graph
- `openspec/changes/p1-c001-flint-auth/` — proposal.md + tasks.md
- `openspec/changes/p1-c002-flint-hooks-standard/` — proposal.md + tasks.md
- `openspec/changes/p1-c003-flint-hooks-durable/` — proposal.md + tasks.md
- `openspec/changes/p1-c004-pg-cron/` — proposal.md + tasks.md
- `openspec/changes/p1-c005-jwt-contract-pin/` — proposal.md + tasks.md (COMPLETE)
- `openspec/changes/p1-c006-vault-kms/` — proposal.md + tasks.md
- `openspec/changes/p1-c007-flint-meta-schema/` — proposal.md + tasks.md
- `openspec/changes/p1-c008-flint-meta-triggers/` — proposal.md + tasks.md
- `openspec/changes/p1-c009-flint-meta-functions/` — proposal.md + tasks.md
- `openspec/changes/p1-c010-flint-meta-agui-descriptor/` — proposal.md + tasks.md
- `openspec/changes/p1-c011-flint-meta-listener-test/` — proposal.md + tasks.md
- `docs/contracts/jwt-contract.md` — JWT claim shape contract (resolves OQ-4 + OQ-5)
- `.kbd-orchestrator/current-waypoint.md` + `current-waypoint.json` — refreshed

## First Change to Apply

Start Batch 1 (all parallel): p1-c001, p1-c004, p1-c006, p1-c007

**p1-c007 is the critical path** — prioritize it.

## Phase Gate

p1-c011 PgListener tests must pass before phase close.
