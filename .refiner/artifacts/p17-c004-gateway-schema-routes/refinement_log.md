# refine-validate — p17-c004-gateway-schema-routes (2026-07-30)
| Constraint | Result | Evidence |
|---|---|---|
| Hexagonal: composition only in gateway | PASS | schema_api composes port+adapter; nothing imports upward |
| No unwrap()/expect() in lib code | PASS | handlers use match/?; expect only in tests |
| 401/403 never leak claims | PASS | error bodies are fixed strings; sub only into ledger param |
| No file > 500 lines | PASS | largest apply.rs 186 lines |
| 500-line/100-line clippy gates | PASS | clippy pedantic -D warnings clean (one justified too_many_lines allow on the linear apply flow) |
| No new deps | PASS | none |
| Never log bearer/claims | PASS | tracing on error paths logs error text only (SQLSTATE-only by adapter contract) |
VERDICT: ALL PASS
