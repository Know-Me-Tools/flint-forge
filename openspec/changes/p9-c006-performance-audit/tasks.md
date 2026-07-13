# p9-c006 Tasks — Performance Audit

## Tasks

- [x] Add `criterion = { version = "0.5", features = ["html_reports"] }` to `[workspace.dependencies]` (dev-dep)
- [x] Create `crates/fdb-reflection/benches/mcp_compiler.rs` — benchmark `McpCompiler::compile()` with 50-table model
- [x] Add `[[bench]]` entry to `crates/fdb-reflection/Cargo.toml`
- [x] Create `crates/fdb-app/benches/design_md_parser.rs` — benchmark `parse_design_md()` with a 9-section fixture
- [x] Add `[[bench]]` entry to `crates/fdb-app/Cargo.toml`
- [ ] Create `perf/k6/` directory with scripts: `rest.js` (components list), `graphql.js` (simple query), `mcp_tools.js` (/mcp/v1/tools), `health.js` (/healthz) — p16-c006: `rest.js`→`components.js` (equivalent, reasonable rename) and `mcp_tools.js`/`health.js` present, but `graphql.js` is genuinely missing. Open debt.
- [x] Create `perf/k6/README.md` — how to run: `BASE_URL=http://localhost:8080 TOKEN=xxx k6 run perf/k6/rest.js`
- [ ] Run `cargo bench -p fdb-reflection` and `cargo bench -p fdb-app` — record baseline numbers — p16-c006: genuinely not done; `docs/performance.md`'s tables are still `TBD` placeholders, not real numbers. Open debt (not run here — a full `cargo bench` execution and recording real numbers is a meaningfully different action than this reconcile pass's file/artifact verification).
- [x] Create `docs/performance.md` — table of baseline P50/P95/P99 from cargo bench; k6 results placeholder — table structure exists; see above re: numbers still TBD
- [x] `cargo clippy --workspace -- -D warnings` clean (benches included in `--all-targets`) — confirmed
- [x] `cargo bench --workspace --no-run` compiles without errors — confirmed just now (11m24s compile, all benches across the workspace produced executables)
