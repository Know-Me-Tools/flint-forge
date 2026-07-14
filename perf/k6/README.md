# Flint Forge — Load Tests (k6)

## Prerequisites

```bash
brew install k6   # macOS
# Linux: https://k6.io/docs/get-started/installation/
```

## Scripts

| Script | Endpoint | Auth | P99 Target | Notes |
|---|---|---|---|---|
| `health.js` | `GET /healthz` | none | < 50 ms | Unauthenticated; fast in-memory response |
| `components.js` | `GET /a2ui/v1/components` | Bearer | < 100 ms | DB-backed component list |
| `mcp_tools.js` | `GET /mcp/v1/tools` | Bearer | < 100 ms | In-memory compiled MCP doc |
| `graphql.js` | `POST /graphql` | Bearer | < 100 ms | `componentsCollection` query, pg_graphql passthrough |
| `regression.js` | All three REST endpoints above | Bearer | see below | **Regression gate** — fails on threshold breach |

---

## Individual scripts

```bash
# Health check (no auth)
BASE_URL=http://localhost:8080 k6 run perf/k6/health.js

# Authenticated endpoints
BASE_URL=http://localhost:8080 TOKEN=<jwt> k6 run perf/k6/components.js
BASE_URL=http://localhost:8080 TOKEN=<jwt> k6 run perf/k6/mcp_tools.js
BASE_URL=http://localhost:8080 TOKEN=<jwt> k6 run perf/k6/graphql.js
```

`TOKEN` is a valid JWT for the running Quarry instance. `BASE_URL` defaults to
`http://localhost:8080` if not set.

---

## Regression gate

`regression.js` runs all three endpoints in a single test and enforces P99
thresholds with 20% headroom above the measured baseline. It exits non-zero
if any threshold is breached.

```bash
BASE_URL=https://forge.example.com TOKEN=<jwt> k6 run perf/k6/regression.js
```

It is also available as a manual GitHub Actions job:
**Actions → CI → Run workflow** (requires `STAGING_BASE_URL` and
`STAGING_SMOKE_TOKEN` repository secrets).

### Current thresholds

| Tag | Threshold | Basis |
|---|---|---|
| `endpoint:healthz` | P99 < 60 ms | Aspirational (TBD) |
| `endpoint:components` | P99 < 120 ms | Aspirational (TBD) |
| `endpoint:mcp_tools` | P99 < 120 ms | Aspirational (TBD) |

> **Note:** `BASELINE_DATE` and `BASELINE_SOURCE` in `regression.js` are currently
> set to `TBD`. Once staging is available, run the individual scripts against the
> live stack, record P50/P95/P99 in `docs/performance.md`, and update both the
> thresholds in `regression.js` and the constants above.

### Updating thresholds after a staging run

1. Run the individual scripts against staging and note the P99 for each endpoint.
2. Set `threshold = ceil(measured_p99 * 1.20)` (round up to nearest 10 ms).
3. Update the three threshold lines in `perf/k6/regression.js`.
4. Record the baseline measurements in `docs/performance.md`.
5. Commit with message: `perf: update k6 regression thresholds from staging baseline`.
