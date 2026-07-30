# refine-validate — p17-c003-provisioner-port-adapter (2026-07-30)

| Constraint | Result | Evidence |
|---|---|---|
| Hexagonal placement | PASS | trait in fdb-ports, DTOs in fdb-domain, impl in fdb-postgres; no adapter import upward |
| No unwrap()/expect() in libs | PASS | adapter uses ?/map_err throughout; expect only in tests |
| thiserror libs / SQLSTATE only | PASS | sqlstate_only() maps every DB error; test asserts statement text never echoed |
| #[non_exhaustive] enums | PASS | no new public enums; reuses BackendError |
| Never log claims/tenant/bearer | PASS | spans: plan_id+namespace only; applied_by deliberately excluded (documented) |
| No file > 500 lines | PASS | provisioner.rs 330, tests 258 |
| No new deps | PASS | none |
| Build health | PASS | cargo check + clippy pedantic clean (2026-07-30) |
VERDICT: ALL PASS
