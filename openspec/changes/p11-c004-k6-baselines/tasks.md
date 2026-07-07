# p11-c004 Tasks — k6 Measured Baselines

## Tasks

- [ ] Add `BASELINE_DATE` and `BASELINE_SOURCE` constants + comment block to `perf/k6/regression.js`
- [ ] Create `perf/results/` directory with `.gitkeep` (result JSON files will be gitignored)
- [ ] Add `perf/results/*.json` to `.gitignore`
- [ ] **If staging is live:** run `perf/k6/health.js`, `components.js`, `mcp_tools.js` against staging; record P50/P95/P99 in `docs/performance.md`; update `regression.js` thresholds to `ceil(measured_p99 * 1.20)` and set `BASELINE_DATE`
- [ ] **If staging is not live:** mark all baseline values as TBD in `docs/performance.md`; add a TODO comment in `regression.js` with the measurement procedure
- [ ] Update `perf/k6/README.md` threshold table to match `regression.js` values
- [ ] `cargo test --workspace` passes (no Rust changes)
