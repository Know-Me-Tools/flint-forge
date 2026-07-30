# refine-validate — p17-c002-domain-ddl-generator (2026-07-30)

Constraints source: .kbd-orchestrator/constraints.md
Artifacts: crates/fdb-domain/src/provision/{mod,spec,plan,validate}.rs,
crates/fdb-app/src/provision/{mod,ddl,hash,tests}.rs, fdb-app Cargo.toml (+sha2)

| Constraint | Result | Evidence |
|---|---|---|
| Hexagonal: no adapter imports in domain/app | PASS | fdb-domain deps: forge-domain+serde only; fdb-app adds sha2 (pure) |
| No unwrap()/expect() in library code | PASS | expect only inside #[cfg(test)] modules; hash.rs surfaces Canonicalize error instead of unwrap; write! result explicitly discarded with comment |
| thiserror in libs | PASS | PlanError (fdb-app); fdb-domain keeps zero-dep manual Error impl (crate has no thiserror dep by design) |
| #[non_exhaustive] on public enums | PASS | ColumnType, OperationKind, SpecError, PlanError |
| #[repr(transparent)] newtype IDs | PASS | Namespace, PlanId, PlanHash |
| Never log claims/tenant ids | PASS | no logging in these modules at all |
| No file > 500 lines | PASS | largest: tests.rs 279 lines, ddl.rs 331 lines |
| No new external dependency | PASS | sha2 already in [workspace.dependencies] |
| missing_docs | PASS | crate-level deny compiles clean |
| clippy pedantic -D warnings | PASS | cargo clippy -p fdb-domain -p fdb-app --all-targets clean (2026-07-30) |

VERDICT: ALL PASS → adversarial-review --mode diff
