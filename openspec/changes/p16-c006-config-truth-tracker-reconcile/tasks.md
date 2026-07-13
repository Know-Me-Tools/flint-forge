# p16-c006 Tasks — Config Truth + Tracker Reconcile

## Tasks

- [x] Make `agui_run` target configurable in `crates/ext-flint-hooks/sql/flint_hooks.sql:156` (default to localhost for dev, override for prod)
- [x] Correct `crates/fdb-reflection/src/compilers/rest/mod.rs:62` doc-comment (CRUD is not `todo!()`)
- [x] Correct `crates/fdb-reflection/src/compilers/rest/mod.rs:120-122` doc-comment (describe post-p16-c001 RLS behavior accurately)
- [x] Correct `crates/fdb-gateway/src/main.rs:56-57` doc-comment re: keto field usage (verify current truth)
- [x] Grep all flagged crates for similar stale-doc drift beyond the two called out; fix each
- [ ] Reconcile `openspec/changes/p9-c001` … `p9-c007` tasks.md checkboxes against shipped artifacts
- [ ] Reconcile `openspec/changes/p10-c001` … `p10-c006` tasks.md checkboxes
- [ ] Reconcile `openspec/changes/p11-c001` … `p11-c006` tasks.md checkboxes
- [ ] Reconcile `openspec/changes/p12-c001`, `p12-c002` tasks.md checkboxes
- [ ] Reconcile `openspec/changes/p14-c001` … `p14-c005` tasks.md checkboxes
- [ ] Leave genuinely-incomplete tasks unchecked and flagged as open debt (no rubber-stamping)
- [ ] Note in this change's completion notes that a final tracker reconcile pass should happen after p16-c007–c009 land
- [ ] `cargo clippy --workspace -- -D warnings` clean
