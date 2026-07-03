# p35-c002 — Fix pre-existing fdb-gateway test debt

## Change ID
`p35-c002-gateway-test-debt`

## Phase
`p3.5-ci-postgres-hardening`

## Goal Mapping
**G3** — green the existing (non-DB) gateway tests so CI can run `cargo test`.

## Depends on
`p35-c001` (CI gate must be clippy-clean first).

## Problem
Two pre-existing issues (confirmed on clean `main`, not introduced by recent work):
1. `keto_sync::tests::keto_sync_config_ignores_non_numeric_env` (`crates/fdb-gateway/src/keto_sync.rs`)
   flakes under parallel test execution: multiple tests `set_var`/`remove_var` the shared
   `KETO_SYNC_INTERVAL_SECS` process env var, so a concurrent test can observe the wrong value.
2. `crates/fdb-gateway/tests/a2ui_seed_test.rs` trips `uninlined_format_args` (2+ sites).

## Scope
- Make the env-dependent `keto_sync` tests deterministic: either serialize them (a shared
  mutex / `serial_test`-style guard) or refactor `keto_sync_config_from_env` tests to parse
  from an injected value rather than mutating the process environment. Prefer the latter
  (no global-state mutation — aligns with the coding-style immutability principle).
- Inline the format args in `a2ui_seed_test.rs`.

## Out of Scope
- The DB-gated a2ui tests' execution (that's c003/c004); this change only fixes the
  non-DB flake + lint.

## Acceptance Criteria
- [ ] `cargo test -p fdb-gateway` (non-DB tests) passes green under default parallel execution, repeatably.
- [ ] `cargo clippy -p fdb-gateway --all-targets -- -D warnings` clean.
- [ ] No production behavior change (test-only + the env-parsing seam if refactored).
