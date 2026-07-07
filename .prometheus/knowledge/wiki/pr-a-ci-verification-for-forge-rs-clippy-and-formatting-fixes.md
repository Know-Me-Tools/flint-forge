---
type: Reference
id: pr-a-ci-verification-for-forge-rs-clippy-and-formatting-fixes
title: PR-A CI Verification for forge-rs Clippy and Formatting Fixes
tags:
- ci
- forge-rs
- clippy
- rustfmt
- bdd
- phase-tracking
links:
- forge-rs-rustfmt-gate-fixed-in-change-green-002
- ci-all-green-plan-for-formatting-forge-rs-and-bdd-failures
sources:
- stdin
timestamp: 2026-07-03T15:48:34.106855+00:00
created_at: 2026-07-03T15:48:34.106855+00:00
updated_at: 2026-07-03T15:48:34.106855+00:00
revision: 0
---

## Phase Context

- Phase: `phase-ci-all-green`
- Project: unspecified
- KBD root: `/Users/gqadonis/Projects/prometheus/prometheus-skill-pack`
- Captured: `2026-07-03T15:44:13Z`
- Position: `phase-ci-all-green`, status `applying`
- Progress: `3/5` changes complete
- PR-A: [PR #23](https://github.com/Prometheus-AGS/prometheus-skill-system/pull/23)

## CI Status

PR #23 is mergeable with `8/9` CI jobs passing.

| Change | Job | CI result |
|---|---|---|
| `001` | Check Formatting | ✅ pass (`17s`) |
| `002` + `003` | forge-rs (`fmt` + `clippy` + `test`) | ✅ pass (`2m56s`) |
| PR-B scope | BDD | ❌ remains failing |

This builds on the rustfmt-only work recorded in [forge-rs rustfmt Gate Fixed in change-green-002](/forge-rs-rustfmt-gate-fixed-in-change-green-002.md). The broader execution plan remains [CI All-Green Plan for Formatting, forge-rs, and BDD Failures](/ci-all-green-plan-for-formatting-forge-rs-and-bdd-failures.md).

## Completed Change: `change-green-003`

`kbd-apply change-green-003` completed and was verified locally and on CI.

The change addressed the second-order work exposed after formatting: `clippy -D warnings` surfaced `12` real lints across `5` forge-rs crates.

Fixed lint categories:

- Unused imports.
- Unused variables.
- `map_or` simplification to `is_some_and`.
- Redundant closure removal.
- `&PathBuf` parameter cleanup to `&Path`.
- Deprecated `tower-http` bearer auth call.
- Dead JSON-RPC protocol field.

## Judgment Calls Confirmed by Review

Two changes were reviewed with `rust-reviewer` before implementation:

- Deprecated `tower-http` bearer auth call:
  - Chosen fix: `#[allow(deprecated)]` with a TODO.
  - Rationale: idiomatic temporary compatibility treatment.
  - Security note preserved in TODO: bearer comparison is not constant-time.
- Dead JSON-RPC field:
  - Chosen fix: `#[allow(dead_code)]` with documentation.
  - Rationale: field is protocol-relevant even if not read by current code.

## Validation Lesson

A CI round-trip was lost because `cargo fmt` was run before clippy edits. Clippy-driven edits reflowed one file, causing the CI formatting step to fail.

Correct validation order for similar Rust CI fixes:

1. Apply clippy fixes.
2. Run `cargo fmt` after clippy changes.
3. Run formatting check.
4. Run clippy.
5. Run tests.

The final amended force-push reran `cargo fmt` and made the forge-rs CI job green.

## Remaining Work

PR-B remains higher-risk and includes:

- `change-green-004`: fix BDD `tsx` loader and exclude `drafts/` from the CI feature glob.
- `change-green-005`: fix `forge validate` to exit non-zero on validation errors.
  - This changes real forge behavior.
  - Immutable BDD tests rule applies.

Open execution decision:

- Merge PR #23 first, then start `/kbd-apply change-green-004`; or
- Stack PR-B on top of PR-A branch before merge.

Open planning decision:

- Include the unbadged `cross-model-qa.yml` workflow in all-green scope or defer it.

# Citations

1. stdin