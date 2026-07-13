# p16-c007 Tasks — 500-Line File-Size Compliance

## Tasks

- [ ] Re-measure all `crates/**/*.rs` line counts after p16-c001–c004 land (list above is pre-remediation)
- [ ] Split `crates/fdb-gateway/src/routes/htmx/renderers.rs` (1267) into a directory module
- [ ] Split `crates/fdb-gateway/src/main.rs` (990) — extract composition sub-steps into `bootstrap/` or similar
- [ ] Split `crates/fdb-gateway/src/routes/a2ui.rs` (802)
- [ ] Split `crates/fdb-realtime/src/listen.rs` (769)
- [ ] Split `crates/fdb-gateway/src/routes/mcp.rs` (732)
- [ ] Split `crates/fke-runtime/src/lib.rs` (674) — coordinate with p16-c003 changes to this file
- [ ] Split `crates/fdb-reflection/src/compilers/mcp.rs` (668)
- [ ] Split `crates/fdb-gateway/src/routes/htmx/mod.rs` (636)
- [ ] Split `crates/fdb-query/src/operator.rs` (632)
- [ ] Split `crates/fdb-reflection/src/compilers/rest/mod.rs` (605) — coordinate with p16-c001 changes to this file
- [ ] Split `crates/fdb-gateway/src/routes/a2a.rs` (552)
- [ ] Split `crates/fdb-app/src/a2ui/design_md_parser.rs` (536)
- [ ] Split `crates/fdb-query/src/plan.rs` (532)
- [ ] Split `crates/forge-cli/src/main.rs` (528)
- [ ] Split `crates/ext-flint-vault/src/lib.rs` (513) — pgrx crate, verify via `cargo pgrx` not `cargo check`
- [ ] Split `crates/fdb-reflection/src/compilers/a2ui.rs` (510)
- [ ] Split `crates/fdb-gateway/src/routes/agui.rs` (508)
- [ ] Re-run the 500-line grep/wc check workspace-wide; confirm zero violations
- [ ] `cargo check --workspace` clean after each file (compile economy)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo test --workspace` passes (no behavior change)
