---
type: Reference
id: phase-ci-all-green-closed-with-validate-skills-badges-green
title: phase-ci-all-green Closed with Validate Skills Badges Green
tags:
- ci
- phase-tracking
- github-actions
- bdd
- forge-rs
- clippy
- rustfmt
- security-hardening
links:
- forge-rs-rustfmt-gate-fixed-in-change-green-002
- pr-a-ci-verification-for-forge-rs-clippy-and-formatting-fixes
- bdd-loader-fixed-and-draft-features-excluded-in-change-green-004
- pr-24-bdd-fixes-complete-phase-ci-all-green-with-all-ci-green
- ci-all-green-plan-for-formatting-forge-rs-and-bdd-failures
sources:
- stdin
- manual:phase-ci-all-green
timestamp: 2026-07-03T17:39:40.693934+00:00
created_at: 2026-07-03T17:39:40.693934+00:00
updated_at: 2026-07-03T17:39:40.693934+00:00
revision: 0
---

## Phase Closure

- Phase: `phase-ci-all-green`
- Project: unspecified
- KBD root: `/Users/gqadonis/Projects/prometheus/prometheus-skill-pack`
- Captured: `2026-07-03T17:35:34Z`
- Final status: `reflect_complete (CLOSED)`
- Goals: `1/1 MET`
- Gate: `0.0`
- Result: all 4 "Validate Skills" README badges are green.

The final verification was against the latest completed `validate.yml` run on `main` at commit `57ec9ef`: status `success`, all 9 jobs green. An initial post-merge check showed 2 red jobs, but that was a transient in-progress snapshot; the final conclusion was delayed until the workflow completed.

## Shipped Changes

| Area | Fix | PR |
|---|---|---|
| `gitleaks` | Replaced license-gated action with free CLI invocation | #22 |
| Check Formatting | Ignored generated `site/`; formatted source files | #23 |
| `forge-rs` fmt, clippy, test | Ran `cargo fmt` and fixed 12 clippy lints across 5 crates | #23 |
| BDD loader and drafts | Switched to `tsx` loader; excluded `drafts/` from executable BDD scope | #24 |
| BDD behavior | Added `#[serde(default)]` on Constitution fields; made `FORGE_BIN` absolute | #24 |

Related implementation records:

- Rust formatting work: [forge-rs rustfmt Gate Fixed in change-green-002](/forge-rs-rustfmt-gate-fixed-in-change-green-002.md)
- Clippy and PR #23 verification: [PR-A CI Verification for forge-rs Clippy and Formatting Fixes](/pr-a-ci-verification-for-forge-rs-clippy-and-formatting-fixes.md)
- BDD loader/draft exclusion: [BDD Loader Fixed and Draft Features Excluded in change-green-004](/bdd-loader-fixed-and-draft-features-excluded-in-change-green-004.md)
- Final BDD fixes and all-green result: [PR #24 BDD Fixes Complete phase-ci-all-green with All CI Green](/pr-24-bdd-fixes-complete-phase-ci-all-green-with-all-ci-green.md)
- Original plan: [CI All-Green Plan for Formatting, forge-rs, and BDD Failures](/ci-all-green-plan-for-formatting-forge-rs-and-bdd-failures.md)

## BDD Root Causes Confirmed by Reproduction

Both BDD failures were diagnosed through reproduction rather than inference:

1. `spawnSync status=null`
   - Not a forge exit-code bug.
   - Cause: CI provided a relative `FORGE_BIN`; BDD steps spawned `forge` from a temporary workdir, so the binary path resolved incorrectly and produced `ENOENT`.
   - Fix: make `FORGE_BIN` absolute in the GitHub Actions workflow.

2. Residual TOML parse failures
   - Cause: TOML key-ordering/config shape trap around missing Constitution fields.
   - Fix: add `#[serde(default)]` while honoring the immutable-tests rule.

## Process Lessons

- A process failure occurred during PR-A: `cargo fmt` was run before clippy edits. Some clippy changes reflowed lines, causing CI's fmt gate to fail after the initial push.
- Impact: one wasted CI cycle and a force-push.
- Standing rule captured: after clippy-driven edits, rerun `cargo fmt` before pushing.

## Known Debt Introduced or Left Open

- `TODO(security)`: forge-mcp bearer auth still uses non-constant-time `tower-http` bearer validation and should be replaced with a constant-time custom validator.
- `#[allow(dead_code)]` remains on a JSON-RPC field.
- A computed-but-unused `_task_description` remains.
- `cross-model-qa.yml` is still red on `main`, but it is not one of the 4 README badge workflows and was intentionally out of scope for this phase.

## Recommended Follow-up Phase

Start `phase-ci-cross-model-qa-and-hardening` to:

1. Green or retire the still-red `cross-model-qa.yml` workflow.
2. Replace deprecated/non-constant-time forge-mcp bearer auth with a constant-time custom validator.
3. Pin a local stable Rust toolchain so local clippy behavior matches CI.

Suggested next command:

```sh
/kbd-assess phase-ci-cross-model-qa-and-hardening
```

# Citations

1. stdin
2. manual:phase-ci-all-green