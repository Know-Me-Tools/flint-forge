---
type: Reference
id: flint-forge-p15-pr-23-vault-backup-restore-regression
title: Flint Forge p15 PR 23 Vault Backup Restore Regression
tags:
- flint-forge
- production-readiness
- postgres
- pgrx
- backup-restore
- ci
- vault-extension
links:
- executor-progress-record-p15-production-readiness-5-of-5
- executor-completion-record-p15-production-readiness-unknown-change
sources:
- stdin
timestamp: 2026-07-16T21:53:24.134804+00:00
created_at: 2026-07-16T21:53:24.134804+00:00
updated_at: 2026-07-16T21:53:24.134804+00:00
revision: 0
---

## Phase Context

- Phase: `p15-v1.0-production-readiness`
- Project: Flint Forge
- KBD root: `/Users/gqadonis/Projects/prometheus/flint-forge/.claude/worktrees/nice-hodgkin-5f0999`
- Captured: `2026-07-16T21:52:14Z`
- Source phase handle: `manual:Flint Forge/p15-v1.0-production-readiness`

## Production Readiness Scope

Phase p15 is intended to close the gap between a compiling/test-passing workspace and a production-ready Flint Forge v1.0. The focus is build integrity, operator tooling, end-to-end validation, documentation accuracy, and production packaging; not new feature development.

Seeded from:

- User directive
- `p14-v1.1.0/reflection.md`

## Planned P0 Blockers

### p15-c001 — Anvil Extension Stabilization

Goal: make all five `ext-flint-*` / `flint_*` pgrx extensions compile and pass `cargo pgrx test` on one supported toolchain.

Required work:

- Unify pgrx version and Postgres target.
- Fix `DatumWithOid` compile error in `ext-flint-meta`.
- Resolve workspace-inheritance misconfiguration for excluded crates.
- Add pgrx CI job in a Linux container.

Gate:

- `cargo pgrx test` passes for all extensions in CI.

### p15-c002 — Migration Integrity

Goal: restore strict linear migration ordering and verify migrations in CI.

Required work:

- Renumber colliding `migrations/0005_*` and `migrations/0006_*` files.
- Add CI step that runs `sqlx migrate run` against an empty Postgres 18 database.

## Session Outcome

- PR created and pushed: `https://github.com/Know-Me-Tools/flint-forge/pull/23`
- Branch: `claude/nice-hodgkin-5f0999`
- Commit: `a033ac5`
- Main implemented fix: `pg_extension_config_dump` handling for:
  - `vault.secrets`
  - `vault.access_log`
- Added regression test script:
  - `backup_restore.sh`

The PR description documents live Postgres 18 reproduction and verification:

- Before the fix: bug confirmed with zero TOC entries.
- After the fix: full backup/restore round-trip preserved data as expected.

## CI Status and Follow-Up

`backup_restore.sh` is not wired into CI yet.

Reason:

- The regression test requires a live Postgres instance with `flint_vault` installed.
- Existing `Dockerfile.vault-check` only compiles/packages the extension; it does not run a live instance for backup/restore validation.

Current handling:

- The missing CI integration was explicitly flagged as a PR checklist item rather than marked complete.

Next actions:

- Await review and CI on PR #23.
- Optional follow-up: wire `backup_restore.sh` into a CI job that provisions live Postgres 18 with `flint_vault` installed.

## Relationship to Other p15 Records

This record provides concrete implementation detail for the same p15 production-readiness phase tracked by broader completion/progress records such as [Executor Progress Record: p15 Production Readiness 5 of 5](/executor-progress-record-p15-production-readiness-5-of-5.md) and the minimal unknown-change completion notices including [Executor Completion Record: p15 Production Readiness Unknown Change](/executor-completion-record-p15-production-readiness-unknown-change.md).

# Citations

1. [1] stdin