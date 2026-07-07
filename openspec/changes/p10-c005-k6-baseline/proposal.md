# p10-c005 — k6 Performance Regression Gate

**Phase:** 10 — Production Launch
**Priority:** P1
**Depends on:** p10-c001, p10-c002 (live staging stack required for baseline measurement)

## Problem

`docs/performance.md` contains aspirational P99 targets, not measured values.
There is no `regression.js` script and no CI job to detect performance regressions
before they reach production.

## Solution

### Part A — `perf/k6/regression.js`

A k6 script that applies the recorded baseline thresholds and fails (`exit 1`)
if P99 exceeds baseline + 20%:

```javascript
import http from 'k6/http';
import { check } from 'k6';

// Baseline P99 targets (replace with measured values from staging run).
// 20% headroom applied: threshold = baseline_p99 * 1.2
export const options = {
  thresholds: {
    'http_req_duration{endpoint:healthz}':    ['p(99)<60'],    // 50ms * 1.2
    'http_req_duration{endpoint:components}': ['p(99)<120'],   // 100ms * 1.2
    'http_req_duration{endpoint:mcp_tools}':  ['p(99)<120'],   // 100ms * 1.2
  },
  vus: 10,
  duration: '30s',
};

export default function () {
  // tag each request with its endpoint name for threshold matching
  http.get(`${__ENV.BASE_URL}/healthz`,
    { tags: { endpoint: 'healthz' } });
  http.get(`${__ENV.BASE_URL}/a2ui/v1/components`,
    { headers: { Authorization: `Bearer ${__ENV.SMOKE_TOKEN}` },
      tags: { endpoint: 'components' } });
  http.get(`${__ENV.BASE_URL}/mcp/v1/tools`,
    { headers: { Authorization: `Bearer ${__ENV.SMOKE_TOKEN}` },
      tags: { endpoint: 'mcp_tools' } });
}
```

### Part B — `performance` job in `ci.yml`

Manual-trigger (`workflow_dispatch`) job; not a required gate (staging dependency):

```yaml
  performance:
    name: k6 Regression
    runs-on: ubuntu-latest
    if: github.event_name == 'workflow_dispatch'
    steps:
      - uses: actions/checkout@v4
      - name: Install k6
        run: |
          sudo apt-get install -y gnupg
          curl -fsSL https://dl.k6.io/key.gpg | sudo gpg --dearmor -o /usr/share/keyrings/k6-archive-keyring.gpg
          echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
          sudo apt-get update && sudo apt-get install -y k6
      - name: Run regression suite
        env:
          BASE_URL: ${{ secrets.STAGING_BASE_URL }}
          SMOKE_TOKEN: ${{ secrets.STAGING_SMOKE_TOKEN }}
        run: k6 run perf/k6/regression.js
```

### Part C — Update `docs/performance.md`

After running the existing k6 scripts against a live staging stack, replace the
aspirational targets table with a `## Measured Baselines` table showing actual
P50/P95/P99 and throughput per endpoint. The `regression.js` thresholds should
be set to `measured_p99 * 1.2` to give 20% headroom.

**Note:** Part C requires a running staging stack. If staging is not yet live
when this change is executed, write the `regression.js` script with placeholder
thresholds and leave a `TODO: update with measured values` comment. The CI job
is defined as `workflow_dispatch` specifically so it runs only when explicitly
triggered against a live target.
