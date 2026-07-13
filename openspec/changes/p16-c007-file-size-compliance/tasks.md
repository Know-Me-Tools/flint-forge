# p16-c007 Tasks — 500-Line File-Size Compliance

## Tasks

- [x] Re-measure all `crates/**/*.rs` line counts after p16-c001–c004 land (list above is pre-remediation) — re-measured 2026-07-13 via `find crates -name "*.rs" | xargs wc -l | sort -rn`; several files grew due to intervening p16 work. Updated list below (new/changed sizes noted; two files newly crossed the 500-line threshold and are added).
- [ ] Split `crates/fdb-gateway/src/routes/htmx/renderers.rs` (1267, unchanged) into a directory module
- [ ] Split `crates/fdb-gateway/src/main.rs` (990 → now 1062 — grew from p16-c003/c004/c006 work) — extract composition sub-steps into `bootstrap/` or similar
- [ ] Split `crates/fdb-gateway/src/routes/a2ui.rs` (802, unchanged)
- [ ] Split `crates/fdb-realtime/src/listen.rs` (769, unchanged)
- [ ] Split `crates/fdb-gateway/src/routes/mcp.rs` (732, unchanged)
- [ ] Split `crates/fke-runtime/src/lib.rs` (674 → now 820 — grew from p16-c003 capability-check changes) — coordinate with p16-c003 changes to this file (p16-c003 is now archived/done)
- [ ] Split `crates/fdb-reflection/src/compilers/mcp.rs` (668, unchanged)
- [ ] Split `crates/fdb-gateway/src/routes/htmx/mod.rs` (636, unchanged)
- [ ] Split `crates/fdb-query/src/operator.rs` (632, unchanged)
- [ ] Split `crates/fdb-reflection/src/compilers/rest/mod.rs` (605 → now 743 — grew from p16-c001 RLS wiring + p16-c006 doc fixes) — coordinate with p16-c001 changes to this file (p16-c001 is now archived/done)
- [ ] Split `crates/fdb-gateway/src/routes/a2a.rs` (552, unchanged)
- [ ] Split `crates/fdb-app/src/a2ui/design_md_parser.rs` (536, unchanged)
- [ ] Split `crates/fdb-query/src/plan.rs` (532, unchanged)
- [ ] Split `crates/forge-cli/src/main.rs` (528 → now 579)
- [ ] Split `crates/ext-flint-vault/src/lib.rs` (513, unchanged) — pgrx crate, verify via `cargo pgrx` not `cargo check`
- [ ] Split `crates/fdb-reflection/src/compilers/a2ui.rs` (510, unchanged)
- [ ] Split `crates/fdb-gateway/src/routes/agui.rs` (508, unchanged)
- [ ] Split `crates/fke-server/src/main.rs` (557) — p16-c006 reconcile: newly over 500 lines (auth middleware from p16-c003), not in the original list. Added.
- [ ] Split `crates/fdb-postgres/src/lib.rs` (541) — newly over 500 lines, not in the original list. Added.
- [ ] Re-run the 500-line grep/wc check workspace-wide; confirm zero violations
- [ ] `cargo check --workspace` clean after each file (compile economy)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo test --workspace` passes (no behavior change)
