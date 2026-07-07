---
type: Reference
id: ci-all-green-plan-for-formatting-forge-rs-and-bdd-failures
title: CI All-Green Plan for Formatting, forge-rs, and BDD Failures
tags:
- ci
- bdd
- forge-rs
- rust-cli
- prettier
- validation
- phase-tracking
sources:
- stdin
- manual:phase-ci-all-green
timestamp: 2026-07-03T14:41:43.715100+00:00
created_at: 2026-07-03T14:41:43.715100+00:00
updated_at: 2026-07-03T14:41:43.715100+00:00
revision: 0
---

## Phase Context

- Phase: `phase-ci-all-green`
- Project: unspecified
- KBD root: `/Users/gqadonis/Projects/prometheus/prometheus-skill-pack`
- Captured: `2026-07-03T14:39:06Z`
- Status: plan complete; execution not started
- Next command: `/kbd-apply change-green-001`

## Local Reproduction Summary

All three failing CI jobs were reproduced locally. The BDD failure is not a simple loader-only issue: after fixing the TypeScript loader locally, the suite exposed real scenario failures caused by `forge validate` returning success on validation errors.

Observed BDD result after switching to `tsx`:

- `13` scenarios total
- `2` passed
- `5` failed
- `6` undefined

Failure breakdown:

- `6` undefined scenarios are all in `tests/features/drafts/okf-wiki-ingest.feature`.
  - The file is a draft with no step definitions.
  - CI incorrectly includes it via the broad `tests/features/**` glob.
- `5` failed scenarios are real failures.
  - `forge validate` prints `Error: invalid language 'cobol'`.
  - The command currently exits with status `0`.
  - Immutable BDD steps require a non-zero exit on validation error.

Per the repository `CLAUDE.md` BDD immutable-tests rule, the step definitions must not be edited to force a pass. The fix belongs in the `forge` binary.

## Ordered Change Plan

| Change | Target job | Risk | Implementation |
|---|---|---:|---|
| `change-green-001` | Check Formatting | Low | Add generated outputs to `.prettierignore`: `site/.docusaurus` and `site/build`; run `prettier --write` on approximately 17 real files. |
| `change-green-002` | `forge-rs-test` | Low | Run `cargo fmt` in vendored `tools/forge-rs`; this directory is committed in-repo and is not a submodule. |
| `change-green-003` | `forge-rs-test` | Medium | Run clippy with `-D warnings` and run tests for `tools/forge-rs`; only formatting has been directly observed so far, so this may expose second-order issues. |
| `change-green-004` | `bdd-test` | Low/Medium | Replace `ts-node/register` with existing `tsx`; exclude `tests/features/drafts/**` from the CI feature glob. |
| `change-green-005` | `bdd-test` | High | Fix the `forge` binary so `forge validate` exits non-zero on validation errors. |

## Risk Notes

### Highest-risk change: `change-green-005`

The `forge validate` exit-code change affects the actual Rust binary and may affect other consumers. Required verification includes:

- `forge-mcp`, which consumes the forge binary.
- `Check Rust CLI` or equivalent Rust CLI checks.
- BDD scenarios that assert validation failure behavior.

The expected behavior after the fix:

```text
forge validate <invalid-input>
# prints validation error
# exits non-zero
```

Current incorrect behavior:

```text
forge validate <invalid-input>
# prints: Error: invalid language 'cobol'
# exits 0
```

### Draft feature handling

The BDD CI glob currently sweeps draft scenarios into the executable suite. `tests/features/drafts/**` should be excluded from CI execution unless matching step definitions and readiness criteria are added.

## PR Strategy

Split into two PRs by risk:

- **PR-A:** `change-green-001`, `change-green-002`, `change-green-003`
  - Formatting and `forge-rs` job fixes.
  - Lower risk and suitable to land first.
- **PR-B:** `change-green-004`, `change-green-005`
  - BDD loader/glob updates plus forge binary validation exit-code fix.
  - Requires careful review and reverification.

Badges are expected to turn green only after the full `validate.yml` workflow passes on `main`; the final merge is the trigger.

## Open Decisions Before Execute

1. **`change-green-005` blast radius:** determine whether making `forge validate` exit non-zero breaks forge callers such as `forge-mcp` or `Check Rust CLI`.
2. **`cross-model-qa.yml`:** currently red but not badged; decide whether to include it in this phase or defer because it is out of scope for the four target badges.

## Generated Artifacts

Planning artifacts were written under:

- `.kbd-orchestrator/phases/phase-ci-all-green/assessment.md`
- `.kbd-orchestrator/phases/phase-ci-all-green/plan.md`
- associated handoff files
- refreshed waypoint

# Citations

1. stdin
2. manual:phase-ci-all-green