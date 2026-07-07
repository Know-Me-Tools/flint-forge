# Flint Forge — Performance Baseline

## Benchmark Results (cargo bench)

Run: `cargo bench --workspace`

### McpCompiler::compile()

| Tables | Median | P95    | Notes                                   |
|--------|--------|--------|-----------------------------------------|
| 10     | TBD    | TBD    | Run `cargo bench -p fdb-reflection`     |
| 25     | TBD    | TBD    | —                                       |
| 50     | TBD    | TBD    | —                                       |
| 100    | TBD    | TBD    | —                                       |

### parse_design_md()

| Input              | Median | P95 | Notes                              |
|--------------------|--------|-----|------------------------------------|
| 9-section DESIGN.md | TBD   | TBD | Run `cargo bench -p fdb-app`       |

---

## Load Test Baselines (k6)

> **Status: TBD** — these values are aspirational targets set before any
> staging measurement. Replace with real P50/P95/P99 figures by running the
> individual scripts against a live staging stack, then update the thresholds
> in `perf/k6/regression.js` to `measured_p99 × 1.20`.

### How to measure

```bash
# Run against staging (requires a running stack from p10-c001/c002)
export BASE_URL=https://forge.example.com
export TOKEN=<staging-jwt>

k6 run --out json=results/healthz.json    perf/k6/health.js
k6 run --out json=results/components.json perf/k6/components.js
k6 run --out json=results/mcp_tools.json  perf/k6/mcp_tools.js
```

Extract P50/P95/P99 from the JSON output:

```bash
# Example: P99 for /healthz
jq '.metrics.http_req_duration.values["p(99)"]' results/healthz.json
```

### Baseline table (fill after first staging run)

| Endpoint                | P50 (ms) | P95 (ms) | P99 (ms) | Threshold | Script              |
|-------------------------|----------|----------|----------|-----------|---------------------|
| `GET /healthz`          | TBD      | TBD      | TBD      | < 60 ms   | `health.js`         |
| `GET /a2ui/v1/components` | TBD    | TBD      | TBD      | < 120 ms  | `components.js`     |
| `GET /mcp/v1/tools`     | TBD      | TBD      | TBD      | < 120 ms  | `mcp_tools.js`      |

Threshold = measured P99 × 1.20 (20 % headroom). Update `perf/k6/regression.js`
after measuring.

---

## Regression Gate (`regression.js`)

`perf/k6/regression.js` is a combined 45-second test (10s ramp-up, 30s hold,
5s ramp-down at 10 VUs) that fails if any per-endpoint P99 threshold is exceeded.

```bash
BASE_URL=https://forge.example.com TOKEN=<jwt> k6 run perf/k6/regression.js
```

It is also triggered via **GitHub Actions → CI → Run workflow** using the
`STAGING_BASE_URL` and `STAGING_SMOKE_TOKEN` repository secrets.

---

## Running Benchmarks

```bash
# All benchmarks
cargo bench --workspace

# Scoped to one crate
cargo bench -p fdb-reflection   # McpCompiler::compile
cargo bench -p fdb-app          # parse_design_md

# Compile only (no execution) — used in CI
cargo bench --workspace --no-run
```

HTML reports are written to `target/criterion/` when criterion's
`html_reports` feature is enabled (already set in `Cargo.toml`).
