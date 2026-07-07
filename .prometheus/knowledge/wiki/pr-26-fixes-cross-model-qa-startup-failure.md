---
type: Reference
id: pr-26-fixes-cross-model-qa-startup-failure
title: 'PR #26 Fixes cross-model-qa Startup Failure'
tags:
- ci-hardening
- github-actions
- cross-model-qa
- actionlint
- phase-tracking
- workflow-dispatch
links:
- forge-rs-stable-toolchain-pin-for-ci-hardening
- ci-cross-model-qa-and-hardening-assessment
- phase-ci-all-green-closed-with-validate-skills-badges-green
sources:
- stdin
- manual:phase-ci-cross-model-qa-and-hardening
timestamp: 2026-07-03T18:30:32.646923+00:00
created_at: 2026-07-03T18:30:32.646923+00:00
updated_at: 2026-07-03T18:30:32.646923+00:00
revision: 0
---

## Phase Context

- Phase: `phase-ci-cross-model-qa-and-hardening`
- KBD root: `/Users/gqadonis/Projects/prometheus/prometheus-skill-pack`
- Captured: `2026-07-03T18:21:47Z`
- Source marker: `manual:phase-ci-cross-model-qa-and-hardening`
- Position: `phase-ci-cross-model-qa-and-hardening`
- Status: `applying`
- Progress: `2/3` changes complete
- Branch/PR: PR #26, `ci/hardening-toolchain-crossqa`

## Completed Change: `change-hard-002`

`kbd-apply change-hard-002` completed and was verified on CI. PR #26 now contains both the stable toolchain pin from [forge-rs Stable Toolchain Pin for CI Hardening](/forge-rs-stable-toolchain-pin-for-ci-hardening.md) and the `cross-model-qa.yml` startup-failure fix.

### CI Result

- PR #26 merge state: `CLEAN`
- `validate.yml`: all `9` jobs green, `0` failing
- Regression guard satisfied: the hardening changes did not break existing validation jobs
- Scope note: `validate.yml` does not run `cross-model-qa.yml`; the workflow-file parse fix was validated separately with `actionlint`

## Root Cause

The `cross-model-qa.yml` workflow failed at startup because the `Post PR comment` step used a multi-line Bash assignment:

```bash
COMMENT="..."
```

Some continuation lines started at column 0 inside a `run: |` YAML block scalar. Those unindented lines terminated the block scalar, so GitHub rejected the workflow file during loading. The resulting symptom was a red `startup_failure` status on every push, even though the workflow was intended for `workflow_dispatch`.

This corresponds to the `cross-model-qa.yml` startup failure identified in [CI Cross-Model QA and Hardening Assessment](/ci-cross-model-qa-and-hardening-assessment.md).

## Fix

The PR rebuilt the PR-comment body generation to avoid YAML block-scalar termination and expression parsing hazards:

- Replaced the fragile multi-line `COMMENT="..."` Bash string with indented `printf` statements.
- Wrote the generated comment to a file and passed it with `--body-file`.
- Moved GitHub Actions expressions such as `github.actor` and the run URL into step `env`.
- Kept the `run` script free of direct `${{ ... }}` syntax.
- Avoided heredoc indentation leakage by using `printf` instead of `<<EOF`.

## Verification

- `actionlint` exits `0`.
- Workflow YAML parses successfully.
- Generated `printf` output has no leading-space indentation leak and renders as clean Markdown.
- PR #26 remains green with all `validate.yml` jobs passing.

## Actionlint-Caught Snags

Two mistakes were caught and fixed before finalizing the change:

1. A literal `${{ }}` appeared inside a Bash comment explaining that `${{ }}` was intentionally avoided. `actionlint` parses GitHub Actions expressions even in comments inside workflow scripts.
2. A duplicate `env:` block was present in the workflow step.

A separate commit-message issue was also handled: because the commit body contained `${{ }}`, invoking `git commit` directly triggered shell `bad substitution`; using `git commit -F <file>` avoided shell interpolation.

## Remaining Scope

This change fixes the parse-time startup failure/red status. A real manual dispatch of `cross-model-qa.yml` still requires owner-provisioned `ANTHROPIC_API_KEY`, which is intentionally out of code scope.

## Next Change

Next planned application: `change-hard-003`.

Planned scope:

- Replace deprecated non-constant-time `forge-mcp` bearer authentication.
- Add a custom `ValidateRequest` using `subtle::ConstantTimeEq`.
- Add the `subtle` dependency.
- Add unit tests.
- Keep `clippy -D warnings` clean.
- Reverify the `:8943` service.
- Route through security review and a separate PR-B.

PR #26 can be merged when ready. PR #25, the docs/state PR from the prior phase, can also be merged to reduce PR-A diff noise; that prior phase is summarized in [phase-ci-all-green Closed with Validate Skills Badges Green](/phase-ci-all-green-closed-with-validate-skills-badges-green.md).

# Citations

1. stdin
2. manual:phase-ci-cross-model-qa-and-hardening