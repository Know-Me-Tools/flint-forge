/**
 * perf/k6/regression.js — Performance regression gate for Flint Forge.
 *
 * Runs a short multi-endpoint load test and FAILS (exit 1) if any P99
 * threshold is exceeded. Designed to run against a live staging stack.
 *
 * Usage:
 *   BASE_URL=https://forge.example.com \
 *   TOKEN=<jwt> \
 *   k6 run perf/k6/regression.js
 *
 * Environment variables:
 *   BASE_URL   fdb-gateway base URL  (default: http://localhost:8080)
 *   TOKEN      JWT bearer token      (required for authenticated endpoints)
 *
 * Thresholds (update after running baseline measurements):
 *   /healthz              P99 < 60 ms   (target 50 ms × 1.20 headroom)
 *   /a2ui/v1/components   P99 < 120 ms  (target 100 ms × 1.20 headroom)
 *   /mcp/v1/tools         P99 < 120 ms  (target 100 ms × 1.20 headroom)
 *
 * TODO (p10-c005): Replace placeholder thresholds above with values measured
 *   against the live staging stack once it is running. Run the individual
 *   scripts (health.js, components.js, mcp_tools.js) to collect P50/P95/P99,
 *   then set threshold = measured_p99 * 1.20, and record the baselines in
 *   docs/performance.md.
 */

import http from 'k6/http';
import { check, group } from 'k6';

const BASE  = __ENV.BASE_URL || 'http://localhost:8080';
const TOKEN = __ENV.TOKEN    || '';

// ── Baseline metadata ─────────────────────────────────────────────────────────
// BASELINE_DATE: date when thresholds were last measured against a live stack.
// BASELINE_SOURCE: the staging URL used for measurement.
// Update these after running: BASE_URL=<staging> TOKEN=<jwt> k6 run regression.js
// See docs/performance.md for the full baseline table and measurement procedure.
const BASELINE_DATE   = 'TBD — run against live staging to measure';
const BASELINE_SOURCE = 'TBD — set BASE_URL env var when running';

// Threshold update procedure:
//   1. Run individual scripts: health.js, components.js, mcp_tools.js against staging
//   2. Record P50/P95/P99 in docs/performance.md baseline table
//   3. Set threshold = ceil(measured_p99 * 1.20) rounded to nearest 5 ms
//   4. Update BASELINE_DATE and BASELINE_SOURCE above
//   5. Commit: "perf: update k6 regression thresholds from <BASELINE_DATE> staging run"

// ── Test configuration ────────────────────────────────────────────────────────
export const options = {
  // Ramp up quickly, hold, then ramp down — gives a meaningful steady-state sample.
  stages: [
    { duration: '10s', target: 10 },   // ramp to 10 VUs
    { duration: '30s', target: 10 },   // hold at 10 VUs (steady-state)
    { duration:  '5s', target:  0 },   // ramp down
  ],

  // ── Per-endpoint P99 thresholds ────────────────────────────────────────────
  // Tags are applied inside the default function; k6 filters thresholds by tag.
  // Format: 'http_req_duration{endpoint:<name>}': ['p(99)<N']
  //
  // Threshold values: target P99 × 1.20 (20% headroom above measured baseline).
  // Replace TBD values with real measurements from the staging run.
  thresholds: {
    // /healthz — unauthenticated, in-memory response. Tight budget.
    'http_req_duration{endpoint:healthz}':    ['p(99)<60'],    // TBD × 1.20

    // /a2ui/v1/components — authenticated, DB-backed component list.
    'http_req_duration{endpoint:components}': ['p(99)<120'],   // TBD × 1.20

    // /mcp/v1/tools — authenticated, in-memory compiled MCP tools document.
    'http_req_duration{endpoint:mcp_tools}':  ['p(99)<120'],   // TBD × 1.20

    // Overall check success rate — no more than 1% of checks should fail.
    checks: ['rate>0.99'],
  },
};

const authHeaders = {
  headers: {
    Authorization: `Bearer ${TOKEN}`,
    'Content-Type': 'application/json',
  },
};

// ── Default function — executed by every VU on every iteration ───────────────
export default function () {
  group('GET /healthz', () => {
    const res = http.get(`${BASE}/healthz`, {
      tags: { endpoint: 'healthz' },
    });
    check(res, { '/healthz status 200': (r) => r.status === 200 });
  });

  if (TOKEN) {
    group('GET /a2ui/v1/components', () => {
      const res = http.get(`${BASE}/a2ui/v1/components`, {
        ...authHeaders,
        tags: { endpoint: 'components' },
      });
      check(res, {
        '/a2ui/v1/components status 200': (r) => r.status === 200,
      });
    });

    group('GET /mcp/v1/tools', () => {
      const res = http.get(`${BASE}/mcp/v1/tools`, {
        ...authHeaders,
        tags: { endpoint: 'mcp_tools' },
      });
      check(res, {
        '/mcp/v1/tools status 200': (r) => r.status === 200,
      });
    });
  }
}

// ── Summary handler — log threshold results ──────────────────────────────────
export function handleSummary(data) {
  const thresholds = data.metrics;
  const lines = [
    '',
    '─── Regression Gate Summary ──────────────────────────────────────',
  ];
  for (const [name, metric] of Object.entries(thresholds)) {
    if (!name.startsWith('http_req_duration') && name !== 'checks') continue;
    const passed = metric.thresholds
      ? Object.values(metric.thresholds).every((t) => !t.ok === false)
      : true;
    lines.push(`  ${passed ? '✓' : '✗'} ${name}`);
  }
  lines.push('─────────────────────────────────────────────────────────────────');
  lines.push('');
  console.log(lines.join('\n'));
  return {};
}
