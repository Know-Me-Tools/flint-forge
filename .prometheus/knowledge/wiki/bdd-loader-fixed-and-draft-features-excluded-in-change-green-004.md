---
type: Reference
id: bdd-loader-fixed-and-draft-features-excluded-in-change-green-004
title: BDD Loader Fixed and Draft Features Excluded in change-green-004
tags:
- ci
- bdd
- cucumber
- tsx
- forge-cli
- phase-tracking
links:
- ci-all-green-plan-for-formatting-forge-rs-and-bdd-failures
- pr-a-ci-verification-for-forge-rs-clippy-and-formatting-fixes
sources:
- stdin
- manual:phase-ci-all-green
timestamp: 2026-07-03T16:06:23.224305+00:00
created_at: 2026-07-03T16:06:23.224305+00:00
updated_at: 2026-07-03T16:06:23.224305+00:00
revision: 0
---

## Phase Context

- Phase: `phase-ci-all-green`
- KBD root: `/Users/gqadonis/Projects/prometheus/prometheus-skill-pack`
- Captured: `2026-07-03T15:50:03Z`
- Position: `phase-ci-all-green`, status `applying`
- Progress: `4/5` changes complete
- Branch: `ci/green-bdd`
- Commit: `ae481b4`

## Completed Change: `change-green-004`

`kbd-apply change-green-004` completed successfully and was verified.

### Loader Fix

- Updated the `package.json` Cucumber script to use:

```sh
NODE_OPTIONS="--import tsx"
```

- Removed reliance on missing `ts-node/register`.
- `tsx` was already available as a `devDependency`.
- Used plain `NODE_OPTIONS` because CI runs on Linux and `cross-env` is not installed.

### Draft Feature Exclusion

- Added `cucumber.mjs` configuration to scope Cucumber `paths` to the two implemented feature areas:
  - `forge-validate`
  - `forge-enrich`
- Intentionally excluded `tests/features/drafts/okf-wiki-ingest.feature`.
- The draft feature was **not deleted**.
- Rationale: draft features without step definitions are allowed under the BDD rule, but must not be included in executable CI scope until implemented.

## Verification Results

The BDD suite now runs without the TypeScript loader failure:

- Scenario count reduced from `13` to `7` by excluding draft-only scenarios.
- Undefined steps reduced from `31` to `0`.
- No immutable step or feature files were modified.

This continues the PR-B BDD work described in [CI All-Green Plan for Formatting, forge-rs, and BDD Failures](/ci-all-green-plan-for-formatting-forge-rs-and-bdd-failures.md), after PR-A stabilized formatting and forge-rs checks as summarized in [PR-A CI Verification for forge-rs Clippy and Formatting Fixes](/pr-a-ci-verification-for-forge-rs-clippy-and-formatting-fixes.md).

## Remaining Work: `change-green-005`

The loader fix exposed `5` real BDD failures in the `forge` binary:

- `forge validate` exits `0` when immutable steps require an exit code matching the validation outcome.
  - Example: validation for unknown language prints an error such as invalid language `cobol`, but still returns success.
- `forge enrich` has incorrect exit-code behavior.
  - Expected: exit `0` on success.
  - Observed: returns `null`.

These are implementation bugs in `forge`, not test-definition problems. Per the immutable-tests rule, fix the `forge` binary rather than changing BDD steps.

Required validation for `change-green-005`:

- Fix `forge validate` and `forge enrich` exit-code behavior.
- Verify all `5` remaining BDD scenarios pass.
- Verify the full `bdd-test` job locally.
- Re-verify `forge-rs-test` and `forge-mcp` because the exit-code changes affect `forge-rs` consumers.
- Push PR-B after local green verification.

Expected final outcome after `change-green-005` merges:

- `validate.yml` passes completely.
- All `4` README badges flip green.

# Citations

1. stdin
2. manual:phase-ci-all-green