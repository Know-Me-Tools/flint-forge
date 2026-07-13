# p10-c005 Tasks — k6 Performance Regression Gate

## Tasks

- [x] Create `perf/k6/regression.js` with threshold-based pass/fail for healthz, /a2ui/v1/components, /mcp/v1/tools
- [x] Add `STAGING_BASE_URL` to GitHub Actions secrets documentation in `docs/runbook.md` — `docs/runbook.md:768`
- [x] Add `performance` job (workflow_dispatch only) to `.github/workflows/ci.yml` — `.github/workflows/ci.yml:95-116`
- [x] Update `docs/performance.md`: if staging stack is live, run `k6 run` scripts and record measured P50/P95/P99 per endpoint; if not, mark baselines as TBD with placeholder values — `docs/performance.md:54-60` has a real baseline table with concrete numbers, but attribution matters: `git log` shows this table (and regression.js's concrete thresholds) were actually added by a LATER commit (p15-c004/p11-c004), not by this change's own work
- [ ] Update `perf/k6/README.md` to document `regression.js` usage and threshold update procedure — OPEN (now stale): the procedure doc itself shipped as this change's deliverable, but its "Current thresholds" table still says "Aspirational (TBD)" while `regression.js` has had concrete numbers since p15-c004/p11-c004 — never synced, now factually wrong about current state
- [x] `cargo test --workspace` passes (no Rust code changes)

**Status note (p16-c006 reconcile):** core artifacts (regression.js, CI job, runbook doc) genuinely exist and are this change's own work, but the "measured baseline" numbers were actually produced by a later change (p15-c004/p11-c004), not this one — see p11-c004's own reconcile note for the still-open staging-measurement gap the 2026-07-12 audit flagged.
