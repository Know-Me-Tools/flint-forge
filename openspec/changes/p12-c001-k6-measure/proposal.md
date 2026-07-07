# p12-c001 — k6 Measured Baselines

**Phase:** 12 — v1.0.0 Release  **Priority:** P0  **Status: DEFERRED**

## Reason for deferral

No live staging stack is available. Per the goals.md fallback policy,
`v1.0.0` will be tagged with aspirational thresholds; this change is
documented as open debt in the release notes.

## What to do when staging becomes available

1. Start the staging stack:
   ```bash
   docker compose -f docker-compose.yml -f docker-compose.staging.yml up -d
   ```
2. Mint a smoke token:
   ```bash
   TOKEN=$(JWT_SECRET=$(cat secrets/jwt_secret.txt) ./scripts/mint_smoke_token.sh)
   ```
3. Run the individual k6 scripts and record P50/P95/P99:
   ```bash
   BASE_URL=https://forge.example.com \
   k6 run --out json=perf/results/healthz.json    perf/k6/health.js
   BASE_URL=https://forge.example.com TOKEN=$TOKEN \
   k6 run --out json=perf/results/components.json perf/k6/components.js
   BASE_URL=https://forge.example.com TOKEN=$TOKEN \
   k6 run --out json=perf/results/mcp_tools.json  perf/k6/mcp_tools.js
   ```
4. Update `docs/performance.md` baseline table with measured values.
5. Set `regression.js` thresholds to `ceil(measured_p99 * 1.20)` and update
   `BASELINE_DATE` + `BASELINE_SOURCE` constants.
6. Run regression gate to confirm pass:
   ```bash
   BASE_URL=https://forge.example.com TOKEN=$TOKEN k6 run perf/k6/regression.js
   ```
7. Commit: `perf: update k6 regression thresholds from <date> staging baseline`
