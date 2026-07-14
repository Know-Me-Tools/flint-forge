# p16-c008 Tasks — Production Operations

## Tasks

**Human/ops involvement required — flag each item below that needs operator action.**

- [x] Clarify with operator: compose or Helm/K8s is the canonical production deploy target
- [x] Extend `.github/workflows/deploy.yml` with a `production` environment option
- [x] Add GitHub Environment protection rules (required reviewers) for production deploys
- [ ] **(operator)** Provision production deploy credentials/secrets — NOT done: this genuinely requires a human with GitHub repo admin access and real production infrastructure; the exact steps are documented in `docs/runbook.md` §13.2, but no agent can execute them (no real credentials exist to provision). Left unchecked deliberately, not rubber-stamped.
- [x] Choose backup approach: `pgBackRest`/`wal-g` sidecar vs. managed-Postgres migration
- [x] Implement the chosen automated backup job with PITR
- [ ] **(operator)** Provision backup storage target credentials — NOT done: requires a real S3-compatible bucket + access/secret keys from a cloud storage provider; documented in `docs/runbook.md` §13.4.2, but no agent can execute this (no real credentials exist to provision). Left unchecked deliberately, not rubber-stamped.
- [ ] **(operator)** Execute and document a restore-from-backup drill; record results in `docs/runbook.md`
- [ ] **(operator)** Run `perf/k6/*.js` scripts against a real staging deployment
- [ ] Commit k6 results to `perf/results/`
- [ ] Update `perf/k6/regression.js` thresholds to `measured_p99 * 1.20` per existing comment
- [x] Decide `llm.enable_background_worker` default (on vs. documented-sync-only)
- [x] Document the LLM-worker-default decision and rationale in `docs/runbook.md`
- [x] Update `docker-compose.prod.yml:57` comment once backups are automated (remove "staging only" caveat if resolved)
- [x] `cargo clippy --workspace -- -D warnings` clean (for any code changes, e.g. worker default)
