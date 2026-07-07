# p9-c006 Tasks — Performance Audit

## Tasks

- [ ] Add `criterion = { version = "0.5", features = ["html_reports"] }` to `[workspace.dependencies]` (dev-dep)
- [ ] Create `crates/fdb-reflection/benches/mcp_compiler.rs` — benchmark `McpCompiler::compile()` with 50-table model
- [ ] Add `[[bench]]` entry to `crates/fdb-reflection/Cargo.toml`
- [ ] Create `crates/fdb-app/benches/design_md_parser.rs` — benchmark `parse_design_md()` with a 9-section fixture
- [ ] Add `[[bench]]` entry to `crates/fdb-app/Cargo.toml`
- [ ] Create `perf/k6/` directory with scripts: `rest.js` (components list), `graphql.js` (simple query), `mcp_tools.js` (/mcp/v1/tools), `health.js` (/healthz)
- [ ] Create `perf/k6/README.md` — how to run: `BASE_URL=http://localhost:8080 TOKEN=xxx k6 run perf/k6/rest.js`
- [ ] Run `cargo bench -p fdb-reflection` and `cargo bench -p fdb-app` — record baseline numbers
- [ ] Create `docs/performance.md` — table of baseline P50/P95/P99 from cargo bench; k6 results placeholder
- [ ] `cargo clippy --workspace -- -D warnings` clean (benches included in `--all-targets`)
- [ ] `cargo bench --workspace --no-run` compiles without errors
