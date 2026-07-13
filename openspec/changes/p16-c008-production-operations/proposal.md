# p16-c008 — Production Operations (Deploy, Backup/PITR, Perf Baselines)

**Phase:** 16 — Production Remediation
**Priority:** P2
**Depends on:** p16-c001, p16-c004 (k6 baselines must be measured against the RLS-fixed, realtime-fixed system to be meaningful)

## What this change delivers

- Production deploy automation (today `.github/workflows/deploy.yml` targets
  `staging` only).
- Automated database backup with point-in-time recovery, replacing the
  "staging only, prefer a managed database" posture of the current prod
  compose DB.
- Captured k6 performance baselines committed to `perf/results/`.
- A decision (and documentation) on whether `ext-flint-llm`'s async worker
  ships enabled by default.

## Problem

`.github/workflows/deploy.yml:9-12` offers only `staging` as an `environment`
choice input, scp/ssh to `STAGING_SSH_*` secrets — there is no production
deploy pipeline; production today is a manual
`docker compose -f ... -f docker-compose.prod.yml` run.

`docker-compose.prod.yml:57` explicitly states its Postgres container is "for
staging only; prefer a managed database" — there is no automated backup job,
WAL archiving, or `pgBackRest`/`wal-g`/`barman` anywhere in the repo. The
runbook (`docs/runbook.md` §5, `:492-535`) documents a **manual** `pg_dump`/
`pg_restore` procedure only.

`perf/k6/*.js` scripts exist but `perf/results/` holds only a `.gitkeep` — no
baseline numbers were ever captured (deferred at v1.0.0 because staging was
unavailable per `.kbd-orchestrator/phases/p12-v1-release/reflection.md`).

## Design — requires human/ops involvement

**This change cannot be completed by an agent alone.** Production credential
provisioning, cloud backup target configuration, and the first
backup/restore drill are exactly the class of hard-to-reverse, externally
visible action an autonomous agent should not perform without a human in the
loop. The agent's role here is to build the automation and documentation; a
human operator must provision credentials, run the first drill, and approve
the first production deploy.

### 1. Production deploy pipeline

Extend `deploy.yml` with a `production` environment option, gated by GitHub
Environment protection rules (required reviewers) so a human approves each
production deploy. Reuse the existing staging deploy mechanics
(scp/ssh + compose) or move to the Helm chart (`deploy/helm/flint-forge/`)
if that's the intended production path — clarify with the operator which
target (compose vs. Helm/K8s) is canonical for production before building.

### 2. Backup automation

Add a scheduled backup job (either `pgBackRest`/`wal-g` sidecar in the compose
stack, or documented migration to a managed Postgres provider with built-in
PITR). Whichever is chosen, it must be **tested** — a restore drill that
proves data comes back, not just that a backup file was written.

### 3. Perf baselines

Run `perf/k6/*.js` against a real staging deployment (now that p16-c001/c004
make the system correctly RLS-enforced and realtime-functional); commit
results to `perf/results/`; wire the `workflow_dispatch` regression gate's
thresholds to `measured_p99 * 1.20` per the existing comment in
`perf/k6/regression.js`.

### 4. LLM async worker default

Decide: enable `llm.enable_background_worker` by default, or keep sync-only
as the supported default and document why (e.g. resource cost, operational
complexity) in `docs/runbook.md`.

## Verification (gate)

- A production deploy runs via CI (with human-approved gate) — not manual
  compose only.
- A restore-from-backup drill is documented **and executed at least once**,
  with results recorded.
- `perf/results/` contains committed baseline numbers; `regression.js`
  thresholds are derived from them, not placeholders.
- `docs/runbook.md` documents the final LLM-worker-default decision and why.
