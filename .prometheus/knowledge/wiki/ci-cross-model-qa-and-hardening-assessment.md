---
type: Reference
id: ci-cross-model-qa-and-hardening-assessment
title: CI Cross-Model QA and Hardening Assessment
tags:
- ci-hardening
- github-actions
- cross-model-qa
- bearer-auth
- forge-rs
- rust-toolchain
- phase-tracking
links:
- phase-ci-all-green-closed-with-validate-skills-badges-green
- forge-rs-rustfmt-gate-fixed-in-change-green-002
sources:
- stdin
timestamp: 2026-07-03T17:55:41.784524+00:00
created_at: 2026-07-03T17:55:41.784524+00:00
updated_at: 2026-07-03T17:55:41.784524+00:00
revision: 0
---

## Phase Context

- Phase: `phase-ci-cross-model-qa-and-hardening`
- Project: unspecified
- KBD root: `/Users/gqadonis/Projects/prometheus/prometheus-skill-pack`
- Captured: `2026-07-03T17:51:43Z`
- Source marker: `manual:phase-ci-cross-model-qa-and-hardening`
- Status: `assessment_complete`
- Step: `0 of 0`

## Lifecycle State

- Completed `kbd-assess` for `phase-ci-cross-model-qa-and-hardening`.
- Assessment, progress, and handoff records were written to `.kbd-orchestrator/phases/phase-ci-cross-model-qa-and-hardening/`.
- Waypoint and project active phase were updated.
- PR #25 (`chore/kbd-phase-ci-all-green`) was opened for KBD lifecycle records from the completed prior phase.
  - Scope: docs/state only.
  - No code changes.
- The previous all-green CI phase is recorded in [phase-ci-all-green Closed with Validate Skills Badges Green](/phase-ci-all-green-closed-with-validate-skills-badges-green.md).

## Assessment Findings

| Gap | Area | Finding | Severity | Notes |
|---|---|---|---|---|
| A | `cross-model-qa.yml` | Startup failure: every push shows `event=push, 0 jobs, conclusion=failure` despite being intended as `on: workflow_dispatch` only. GitHub rejects the workflow file at load time. | LOW | Workflow is unbadged and gates nothing, so the red status is cosmetic. |
| B | Bearer auth | `forge-mcp/lib.rs:83` uses deprecated, non-constant-time `ValidateRequestHeaderLayer::bearer`. | MEDIUM | Proposed fix: custom `ValidateRequest` implementation using `subtle::ConstantTimeEq`; `subtle` is not yet a dependency. |
| C | Toolchain pin | No `rust-toolchain.toml`. Local default is nightly while CI uses stable. | LOW | This exact mismatch caused a CI round trip in the prior phase. |

## Important Assessment Notes

- The local PyYAML report of a `line 130 parse error` was not accepted as the root cause for `cross-model-qa.yml`.
  - GitHub Actions parsing is more lenient than PyYAML.
  - Treat the PyYAML result as likely false positive until confirmed.
- Open question `OQ-A1`: read GitHub's actual workflow startup error through `actionlint` and/or the GitHub Actions UI before fixing Gap A.
- Open question `OQ-A2`: decide whether to fix or retire `cross-model-qa`.
  - Current recommendation: fix it, because it is useful as an anti-sycophancy secondary-review tool.
- Open question `OQ-A3`: confirm whether `ANTHROPIC_API_KEY` is configured.
  - If absent, a syntax-fixed workflow still cannot complete a review.

## Regression Guard

- `validate.yml` is currently all-green.
- Any changes, especially Gap B changes in `forge-rs`, must keep `validate.yml` green.
- Related prior forge-rs formatting and gate work is recorded in [forge-rs rustfmt Gate Fixed in change-green-002](/forge-rs-rustfmt-gate-fixed-in-change-green-002.md).

## Recommended Next Step

Run:

```sh
/kbd-plan phase-ci-cross-model-qa-and-hardening
```

Before planning, resolve the scope-changing open questions:

1. Use `actionlint` and/or GitHub Actions UI to obtain the actual `cross-model-qa.yml` startup error.
2. Decide whether `cross-model-qa` should be fixed or retired.
3. Check whether `ANTHROPIC_API_KEY` is set.

# Citations

1. stdin