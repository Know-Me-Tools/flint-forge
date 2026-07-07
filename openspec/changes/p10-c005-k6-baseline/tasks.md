# p10-c005 Tasks — k6 Performance Regression Gate

## Tasks

- [ ] Create `perf/k6/regression.js` with threshold-based pass/fail for healthz, /a2ui/v1/components, /mcp/v1/tools
- [ ] Add `STAGING_BASE_URL` to GitHub Actions secrets documentation in `docs/runbook.md`
- [ ] Add `performance` job (workflow_dispatch only) to `.github/workflows/ci.yml`
- [ ] Update `docs/performance.md`: if staging stack is live, run `k6 run` scripts and record measured P50/P95/P99 per endpoint; if not, mark baselines as TBD with placeholder values
- [ ] Update `perf/k6/README.md` to document `regression.js` usage and threshold update procedure
- [ ] `cargo test --workspace` passes (no Rust code changes)
