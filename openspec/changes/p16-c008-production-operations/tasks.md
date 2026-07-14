# p16-c008 Tasks — Production Operations

## Tasks

**Human/ops involvement required — flag each item below that needs operator action.**

- [x] Clarify with operator: compose or Helm/K8s is the canonical production deploy target
- [ ] Extend `.github/workflows/deploy.yml` with a `production` environment option
- [ ] Add GitHub Environment protection rules (required reviewers) for production deploys
- [ ] **(operator)** Provision production deploy credentials/secrets
- [ ] Choose backup approach: `pgBackRest`/`wal-g` sidecar vs. managed-Postgres migration
- [ ] Implement the chosen automated backup job with PITR
- [ ] **(operator)** Provision backup storage target credentials
- [ ] **(operator)** Execute and document a restore-from-backup drill; record results in `docs/runbook.md`
- [ ] **(operator)** Run `perf/k6/*.js` scripts against a real staging deployment
- [ ] Commit k6 results to `perf/results/`
- [ ] Update `perf/k6/regression.js` thresholds to `measured_p99 * 1.20` per existing comment
- [ ] Decide `llm.enable_background_worker` default (on vs. documented-sync-only)
- [ ] Document the LLM-worker-default decision and rationale in `docs/runbook.md`
- [ ] Update `docker-compose.prod.yml:57` comment once backups are automated (remove "staging only" caveat if resolved)
- [ ] `cargo clippy --workspace -- -D warnings` clean (for any code changes, e.g. worker default)
