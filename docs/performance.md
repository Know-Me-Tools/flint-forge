# Flint Forge — Performance Baseline

## Benchmark Results (cargo bench)

Run: `cargo bench --workspace`

> **Status: local measurement — p16-c006.** Measured via `cargo bench -p
> fdb-reflection` / `cargo bench -p fdb-app` (100 samples each, criterion
> default). Median/P95 computed from the raw per-iteration sample data in
> `target/criterion/*/new/sample.json`. Re-measure on CI/release hardware if
> these numbers are used for capacity planning.

### McpCompiler::compile()

| Tables | Median   | P95      | Notes                                |
|--------|----------|----------|----------------------------------------|
| 10     | 930 µs   | 1.17 ms  | `cargo bench -p fdb-reflection`      |
| 25     | 2.15 ms  | 2.87 ms  | —                                     |
| 50     | 6.40 ms  | 12.07 ms | —                                     |
| 100    | 12.83 ms | 17.52 ms | —                                     |

### parse_design_md()

| Input                | Median   | P95      | Notes                        |
|-----------------------|----------|----------|--------------------------------|
| 9-section DESIGN.md   | 28.07 µs | 35.87 µs | `cargo bench -p fdb-app`     |

---

## Load Test Baselines (k6)

> **Status: local Colima baseline — p15-c004.** These figures were measured
> against a local Docker Compose stack (`docker compose up -d`) on a Colima
> macOS host. They are conservative enough to stay green on modest CI runners
> and should be re-measured against a production-like staging host once one is
> provisioned.

### How to measure

```bash
# Local stack (default)
docker compose up -d

# Or against staging
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

### Baseline table

| Endpoint                  | P50 (ms) | P95 (ms) | P99 (ms) | Threshold | Script          |
|---------------------------|----------|----------|----------|-----------|-----------------|
| `GET /healthz`            | 12       | 22       | 42       | < 55 ms   | `health.js`     |
| `GET /a2ui/v1/components` | 35       | 72       | 108      | < 135 ms  | `components.js` |
| `GET /mcp/v1/tools`       | 18       | 38       | 76       | < 95 ms   | `mcp_tools.js`  |

Threshold = measured P99 × 1.20 (20 % headroom), rounded up to the nearest
5 ms. Update `perf/k6/regression.js` after each re-measurement.

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
