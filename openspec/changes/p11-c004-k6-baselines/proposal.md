# p11-c004 — k6 Measured Baselines

**Phase:** 11 — API Stability  **Priority:** P1  **Depends on:** live staging stack

## Problem

`perf/k6/regression.js` thresholds are aspirational (`// TBD × 1.20`).
`docs/performance.md` baseline table has all TBD values. There is no
`baseline_date` annotation to track when thresholds were last measured.

## Solution

### Part A — Annotation (can be done now, no staging required)

Add `baseline_date` and `baseline_source` comments to `regression.js`:

```js
// Baselines measured: TBD (requires live staging stack)
// Update procedure: run perf/k6/health.js, components.js, mcp_tools.js against
//   staging; set threshold = ceil(measured_p99 * 1.20); update baseline_date.
const BASELINE_DATE = 'TBD';
```

### Part B — Measurement (requires live staging)

Run each k6 script against the staging stack and record results:

```bash
BASE_URL=https://forge.example.com TOKEN=<jwt> \
  k6 run --out json=perf/results/healthz.json perf/k6/health.js

BASE_URL=https://forge.example.com TOKEN=<jwt> \
  k6 run --out json=perf/results/components.json perf/k6/components.js

BASE_URL=https://forge.example.com TOKEN=<jwt> \
  k6 run --out json=perf/results/mcp_tools.json perf/k6/mcp_tools.js
```

Extract P99:

```bash
jq '.metrics.http_req_duration.values["p(99)"]' perf/results/healthz.json
```

Update `docs/performance.md` baseline table with measured P50/P95/P99.
Update thresholds in `regression.js` to `ceil(measured_p99 * 1.20)`.

### Scope decision

If staging is unavailable when this change is executed, scope to Part A only:
add the annotation + `perf/results/` directory + `.gitkeep`. Record this as a
known open debt in the commit message.
