---
type: Reference
id: pr-24-bdd-fixes-complete-phase-ci-all-green-with-all-ci-green
title: 'PR #24 BDD Fixes Complete phase-ci-all-green with All CI Green'
tags:
- ci
- bdd
- forge-rs
- github-actions
- phase-tracking
- cucumber
links:
- bdd-loader-fixed-and-draft-features-excluded-in-change-green-004
- forge-rs-rustfmt-gate-fixed-in-change-green-002
- pr-a-ci-verification-for-forge-rs-clippy-and-formatting-fixes
- ci-all-green-plan-for-formatting-forge-rs-and-bdd-failures
sources:
- stdin
- manual:phase-ci-all-green
timestamp: 2026-07-03T17:24:38.122688+00:00
created_at: 2026-07-03T17:24:38.122688+00:00
updated_at: 2026-07-03T17:24:38.122688+00:00
revision: 0
---

## Phase Context

- Phase: `phase-ci-all-green`
- KBD root: `/Users/gqadonis/Projects/prometheus/prometheus-skill-pack`
- Captured: `2026-07-03T17:20:09Z`
- Source: `manual:phase-ci-all-green`
- Status: `execute_complete`
- Progress: `5/5` changes complete

## Completed Change: `change-green-005`

`kbd-apply change-green-005` completed and was verified. This was the final change needed after [BDD Loader Fixed and Draft Features Excluded in change-green-004](/bdd-loader-fixed-and-draft-features-excluded-in-change-green-004.md).

### Root Cause 1: `spawnSync status=null`

- Affected: `4` BDD scenarios.
- Symptom: Node `spawnSync` returned `status=null`.
- Diagnosis: this was a spawn error, not a forge exit-code bug.
- Cause:
  - BDD steps spawn `forge` with `cwd` set to a temporary workdir.
  - CI provided a relative `FORGE_BIN`.
  - The relative path resolved against the temp workdir, not the repository root.
  - Result: `ENOENT`, producing `status=null`.
- Fix: make `FORGE_BIN` absolute in the GitHub Actions workflow using `${{ github.workspace }}/...`.

### Root Cause 2: `missing field required_skills`

- Affected: `2` BDD scenarios.
- Symptom: TOML config parsing failed with `missing field required_skills`.
- Cause:
  - The immutable test step's TOML placed `required_skills=[]` after a `[[forbidden_patterns]]` table.
  - TOML therefore bound `required_skills` inside the table instead of at the top level.
- Fix in forge, not the test:
  - Added `#[serde(default)]` to `Constitution` collection fields.
  - This is additive parser leniency for omitted collections.
  - It does not reject any previously valid config.

## Verification

Local verification after the fixes:

- BDD: `7/7` scenarios passed.
- BDD steps: `28/28` steps passed.
- `forge-rs` gates remain green.
- `forge-mcp` still builds.
- No immutable BDD test steps were modified.

Diagnostic method:

- Reproduced `spawnSync` exactly to confirm `status=null` was caused by spawn failure/`ENOENT`, not process exit behavior.
- Isolated the TOML parsing issue with a temporary throwaway test, then removed it cleanly.

## CI and PR Status

| PR / Scope | Job | Result |
|---|---|---|
| PR #23, merged | Check Formatting | ✅ pass |
| PR #23, merged | `forge-rs` fmt + clippy + test | ✅ pass |
| PR #24, open | BDD tests | ✅ pass |
| Other validation | gitleaks, Rust CLI, hooks-integrity, sycophancy, skill-collision, AgentSkills | ✅ pass |

- PR #24: <https://github.com/Prometheus-AGS/prometheus-skill-system/pull/24>
- PR #24 has all `9` jobs green.
- PR #24 `mergeStateStatus=CLEAN`.
- The overall phase builds on prior formatting and `forge-rs` work from [forge-rs rustfmt Gate Fixed in change-green-002](/forge-rs-rustfmt-gate-fixed-in-change-green-002.md) and [PR-A CI Verification for forge-rs Clippy and Formatting Fixes](/pr-a-ci-verification-for-forge-rs-clippy-and-formatting-fixes.md), following the original [CI All-Green Plan for Formatting, forge-rs, and BDD Failures](/ci-all-green-plan-for-formatting-forge-rs-and-bdd-failures.md).

## Remaining Actions

- Merge PR #24 to update `main`.
- Once `validate.yml` passes on `main`, the four workflow-status "Validate Skills" badges should turn green.
- Optional: run `/kbd-reflect phase-ci-all-green` to close the phase.
- Open follow-up decision: `cross-model-qa.yml` remains red but is unbadged and outside the badge-focused CI scope.

# Citations

1. stdin
2. manual:phase-ci-all-green