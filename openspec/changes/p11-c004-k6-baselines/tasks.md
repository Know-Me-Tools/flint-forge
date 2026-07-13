# p11-c004 Tasks — k6 Measured Baselines

## Tasks

- [x] Add `BASELINE_DATE` and `BASELINE_SOURCE` constants + comment block to `perf/k6/regression.js` — `perf/k6/regression.js:36-37`
- [x] Create `perf/results/` directory with `.gitkeep` (result JSON files will be gitignored) — exists
- [x] Add `perf/results/*.json` to `.gitignore` — `.gitignore:69`
- [ ] **If staging is live:** run `perf/k6/health.js`, `components.js`, `mcp_tools.js` against staging; record P50/P95/P99 in `docs/performance.md`; update `regression.js` thresholds to `ceil(measured_p99 * 1.20)` and set `BASELINE_DATE` — OPEN: **not done against staging**. `regression.js`'s `BASELINE_SOURCE = 'local Docker Compose stack (Colima) — re-measure on staging'` and the numbers in `docs/performance.md` are attributed to a LOCAL run (matches p15-c004), not a live staging measurement. `perf/results/` contains only `.gitkeep` — no actual result JSON anywhere in the repo. This is exactly the gap the 2026-07-12 audit flagged ("no k6 baselines; regression gate deferred") — confirmed still true.
- [ ] **If staging is not live:** mark all baseline values as TBD in `docs/performance.md`; add a TODO comment in `regression.js` with the measurement procedure — OPEN: neither branch was cleanly executed — `docs/performance.md` has concrete (non-TBD) numbers mislabeled as a baseline rather than "TBD" as this fallback specifies. Do not check off — this is the confirmed gap.
- [ ] Update `perf/k6/README.md` threshold table to match `regression.js` values — OPEN: stale/contradictory — `perf/k6/README.md:52-58` still shows thresholds `60/120/120ms` labeled "Aspirational (TBD)" while `regression.js` actually has `55/135/95ms` with a concrete `BASELINE_DATE = '2026-07-08'`. The README was never synced to the actual values.
- [x] `cargo test --workspace` passes (no Rust changes)

**Status note (p16-c006 reconcile):** only 3/7 tasks are genuinely shippable — this is the exact change the 2026-07-12 audit called out, and the gap is confirmed real: the "measured baseline" work that does exist was performed under a DIFFERENT change (p15-c004) using a local Colima stack, not this change's staging requirement. Remains open debt; see p16-c006's own note about a final reconcile pass after p16-c007–c009 land.
