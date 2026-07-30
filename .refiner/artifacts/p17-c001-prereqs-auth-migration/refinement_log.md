# refine-validate — p17-c001-prereqs-auth-migration (2026-07-30)

Constraints source: .kbd-orchestrator/constraints.md
Artifacts: migrations/0015_flint_schema_provisioning.sql, .env.example,
docs/runbook.md §14, docs/ANON-SERVICE-ROLE-KEYS.md,
crates/fdb-gateway/tests/provisioning_prereqs.rs

| Constraint | Result | Evidence |
|---|---|---|
| No adapter import from domain/app crates | PASS | only a gateway test target touched |
| No unwrap()/expect() in library crates | PASS | expect only in the test (existing `#![allow(clippy::expect_used)]` test convention, matching rest_rls_isolation.rs) |
| Never log JWT/claims/tenant ids | PASS | test logs env-var NAMES only; doc comment repeats the rule |
| No file > 500 lines | PASS | largest new file 143 lines |
| No new dependency without workspace check | PASS | zero new deps |
| pgrx crates untouched by plain cargo | PASS | no ext-flint-* changes |
| No clippy allow without justification | PASS | one allow(expect_used) in tests, per repo test convention |
| Build health | PASS | cargo check -p fdb-gateway --tests clean (1m56s, 2026-07-30) |

VERDICT: ALL PASS → proceed to adversarial-review --mode diff
