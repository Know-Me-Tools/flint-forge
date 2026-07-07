# p9-c006 — Performance Audit + Benchmarks

**Phase:** 9 — Production Hardening
**Priority:** P1
**Depends on:** none (cargo bench is independent; k6 needs live server)

## What this change delivers

- `criterion` benchmarks for `McpCompiler::compile()` and `parse_design_md()`
- `perf/k6/` load test scripts for `fdb-gateway` REST endpoints
- `docs/performance.md` — baseline measurements and optimisation notes

## Design

### `cargo bench` targets

**`crates/fdb-reflection/benches/mcp_compiler.rs`**
```rust
use criterion::{criterion_group, criterion_main, Criterion};
use fdb_reflection::compilers::mcp::McpCompiler;

fn bench_mcp_compile(c: &mut Criterion) {
    let model = make_large_model(50); // 50 tables, 5 functions, 10 views
    c.bench_function("McpCompiler::compile 50 tables", |b| {
        b.iter(|| McpCompiler::compile(&model))
    });
}
```

**`crates/fdb-app/benches/design_md_parser.rs`**
```rust
fn bench_parse_design_md(c: &mut Criterion) {
    let md = include_str!("../test_fixtures/large_design.md");
    c.bench_function("parse_design_md 9 sections", |b| {
        b.iter(|| fdb_app::a2ui::parse_design_md(md).unwrap())
    });
}
```

### New workspace dep

```toml
criterion = { version = "0.5", features = ["html_reports"] }
```
(dev-dep only)

### k6 scripts (`perf/k6/`)

```javascript
// perf/k6/rest.js — GET /a2ui/v1/components load test
import http from 'k6/http';
export const options = { vus: 50, duration: '30s',
  thresholds: { 'http_req_duration{status:200}': ['p(99)<100'] } };
export default function () {
  http.get(`${__ENV.BASE_URL}/a2ui/v1/components`,
    { headers: { Authorization: `Bearer ${__ENV.TOKEN}` } });
}
```

Similar scripts for `/graphql`, `/mcp/v1/tools`, `/healthz`.
