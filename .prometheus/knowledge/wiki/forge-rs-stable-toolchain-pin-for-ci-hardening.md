---
type: Reference
id: forge-rs-stable-toolchain-pin-for-ci-hardening
title: forge-rs Stable Toolchain Pin for CI Hardening
tags:
- ci-hardening
- rust-toolchain
- forge-rs
- stable-rust
- github-actions
- cross-model-qa
- phase-tracking
links:
- ci-cross-model-qa-and-hardening-assessment
- phase-ci-all-green-closed-with-validate-skills-badges-green
- forge-rs-rustfmt-gate-fixed-in-change-green-002
sources:
- stdin
timestamp: 2026-07-03T18:18:28.468074+00:00
created_at: 2026-07-03T18:18:28.468074+00:00
updated_at: 2026-07-03T18:18:28.468074+00:00
revision: 0
---

## Phase Context

- Phase: `phase-ci-cross-model-qa-and-hardening`
- Project: unspecified
- KBD root: `/Users/gqadonis/Projects/prometheus/prometheus-skill-pack`
- Captured: `2026-07-03T18:08:54Z`
- Source marker: `manual:phase-ci-cross-model-qa-and-hardening`
- Branch: `ci/hardening-toolchain-crossqa`
- Status: `applying`
- Progress: `1/3` changes complete
- Related assessment: [CI Cross-Model QA and Hardening Assessment](/ci-cross-model-qa-and-hardening-assessment.md)

## Completed Change

`kbd-apply change-hard-001` completed and was verified.

### Result

- Commit: `94a0edf`
- Added `tools/forge-rs/rust-toolchain.toml`.
- Toolchain pinned to stable with required components:
  - `rustfmt`
  - `clippy`
- Active toolchain now resolves to stable for `forge-rs`:

```text
rustup show active-toolchain
stable-aarch64-apple-darwin (overridden by rust-toolchain.toml)
```

## Verification

All three local CI gates pass on stable Rust (`rustc 1.96.0`):

```text
cargo fmt --check                         # exit 0
cargo clippy --all --all-features -D warnings  # exit 0
cargo test --all                          # exit 0
```

This confirms the previous `forge-rs` work from [phase-ci-all-green Closed with Validate Skills Badges Green](/phase-ci-all-green-closed-with-validate-skills-badges-green.md) is green on the same stable toolchain used by CI, not only on a local nightly toolchain. It closes the nightly-vs-stable gap that caused a CI round trip in the prior phase. The related formatting/clippy foundation includes [forge-rs rustfmt Gate Fixed in change-green-002](/forge-rs-rustfmt-gate-fixed-in-change-green-002.md).

## Next Work

- Next KBD item: `change-hard-002`.
- Planned fix: repair the `cross-model-qa.yml` block-scalar parse error.
  - Rebuild the `Post PR comment` step using a heredoc plus `--body-file`.
  - Verify the workflow with `actionlint`.
- Follow-up: `change-hard-003`, implementing constant-time bearer authentication with security review.
- PR grouping: `change-hard-001` and `change-hard-002` are planned to ship together in PR-A.

# Citations

1. stdin