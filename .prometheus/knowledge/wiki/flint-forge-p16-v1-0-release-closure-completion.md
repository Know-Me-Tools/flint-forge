---
type: Reference
id: flint-forge-p16-v1-0-release-closure-completion
title: Flint Forge p16 v1.0 Release Closure Completion
tags:
- flint-forge
- release-closure
- operator-guide
- security-disclosure
- postgres-backup
- phase-tracking
sources:
- stdin
- manual:Flint Forge/p16-v1.0-release-closure
timestamp: 2026-07-16T21:50:18.554767+00:00
created_at: 2026-07-16T21:50:18.554767+00:00
updated_at: 2026-07-16T21:50:18.554767+00:00
revision: 0
---

## Phase Context

- Project: Flint Forge
- Phase: `p16-v1.0-release-closure`
- KBD root: `/Users/gqadonis/Projects/prometheus/flint-forge`
- Captured: `2026-07-16T21:48:12Z`
- Status: executing; changes `6/6` complete
- Seeded from:
  - `p15-v1.0-production-readiness/reflection.md` → Recommended next phase: P0 v1.0 release closure
  - `handoffs/reflect-to-next.json` → `next_phase_hint: v1.0-release-closure`

## Phase Goal

Ship Flint Forge v1.0 by turning the verified-green p15 workspace into a tagged, packaged, released artifact with an operator handoff, while paying down process debt carried from p15.

## Inherited Debt

Carried from p15 `handoffs/reflect-to-next.json` `open_debt`, plus one p15 closure discovery:

1. No artifact-refiner QA logs exist for p15 changes; `.refiner/` was absent.
2. `p15.total_waits` was `6`, exceeding the documented 3-wait budget.
3. k6 baselines were local Colima numbers, not production-like staging numbers.
4. Native KBD changes were tracked in `progress.json` but not archived under `.kbd-orchestrator/changes/archive/<date>-<id>/`.
5. KBD position files drifted badly out of sync during p15:
   - `position.json` was 11 phases stale.
   - `position-reminder.txt` was 12 phases stale.
   - New sessions were incorrectly told they were in `p3-auth-rls-keto`.

## Completed Changes

`kbd-apply` completed and archived all remaining p16 changes:

- `p16-c005-docs-reality-reconciliation`
- `p16-c004-security-disclosure`
- `p16-c006-selfhost-operator-guide`

Overall p16 completion: `6/6` changes complete.

## Change Details

### `p16-c005-docs-reality-reconciliation`

Documentation and comments were reconciled with current implementation reality:

- Fixed stale `todo!()` claim in `rest/mod.rs`.
- Updated README subscription claims to match the post-`p16-c002` default behavior.
- Removed stale wording in `mounts_reflection_router.rs` that still described handlers as panicking on `todo!()`.

### `p16-c004-security-disclosure`

Security disclosure process was established using GitHub Security Advisories:

- User selected GitHub Security Advisories as the disclosure mechanism.
- Repo setting was checked first and found disabled: `{"enabled":false}`.
- Setting was only changed after confirmation because it is repo-wide.
- GitHub Security Advisories were enabled and verified: `{"enabled":true}`.
- Added `SECURITY.md` with realistic commitments:
  - 5-day acknowledgement target.
  - Biweekly update cadence.
  - No fabricated or unmeetably strict SLA.
- Added `SUPPORT.md`.
- Added `CONTRIBUTING.md`.

### `p16-c006-selfhost-operator-guide`

The operator guide required tested restore behavior, not a theorized path. Validation was performed against live PostgreSQL infrastructure:

- Docker Desktop Kubernetes was unavailable.
- After user confirmation, plain Docker was used instead.
- A live PostgreSQL 18 container was started.
- All 11 migrations were applied.
- A `flint_vault` secret was created.
- A real `pg_dump` backup was taken.
- Backup was restored into a fresh PostgreSQL instance.
- Both failure and success paths for the DEK were validated.

## Findings and Follow-Ups

### P0: `vault.secrets` silently omitted from `pg_dump`

A real backup/restore gap was discovered during operator-guide validation:

- `vault.secrets` rows are silently excluded from `pg_dump` output.
- The rows are absent from the dump, not merely unreadable after restore.
- Root cause: the extension did not opt into `pg_extension_config_dump()`.
- A tested workaround was documented in the operator guide.
- The extension-code fix was spawned as a follow-up because `p16-c006` was documentation-scoped.

Severity: P0; outside docs-only phase scope.

### P0: gateway startup `/healthz` blocked by missing `flint_meta.views()`

Final `/healthz` validation did not pass because of a pre-existing bug:

- Startup fails due to missing `flint_meta.views()`.
- The bug reproduces on a completely fresh, non-restored database.
- Therefore it is unrelated to restore correctness.
- The operator guide discloses the criterion as not met instead of hiding the failure.
- A separate follow-up was spawned.

Severity: P0; outside docs-only phase scope.

## Release Blocker

The remaining release-action blocker is deciding whether to:

- re-tag `v1.0.0`, or
- release `v1.0.1`.

This decision should be resolved before or during `/kbd-reflect p16-v1.0-release-closure`.

## Next Step

Run:

```bash
/kbd-reflect p16-v1.0-release-closure
```

# Citations

1. stdin
2. manual:Flint Forge/p16-v1.0-release-closure