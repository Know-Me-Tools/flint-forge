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

## Design — corrected 2026-07-14

**The original assumption below (single SSH-reachable host, S3 backup
target, "run k6 against a real staging deployment") was wrong for this
org's actual infrastructure and blocked the change on manual operator
steps that were never going to be reachable.** The real target is a
shared, multi-tenant AKS cluster (`main`, resource group `prometheus-rg`)
that already runs ArgoCD in-cluster for several other projects on Azure —
"production" is deployed once per client onto that cluster, not to one
fixed host, and there is no S3 or separate staging host anywhere in this
org's stack. Once the real target was identified, every item below became
independently automatable using existing, real infrastructure (a shared
GitHub OIDC → Azure AD app already used by 6 other projects, Azure AD
Workload Identity for storage access, the docker-compose stack as the only
real pre-production environment) — see `docs/runbook.md` §13 for the
as-built architecture. No human operator action is required for the system
to function; §13.7 documents the one remaining manual step (onboarding a
*new* tenant), which is inherent to a multi-tenant system, not a gap.

### 1. Production deploy pipeline

`.github/workflows/deploy-aks.yml` builds images and pushes them to the
shared ACR via GitHub OIDC (no stored credentials), then commits the new
tag into `deploy/helm/flint-forge/values-<tenant>.yaml` — that commit is
the deploy. ArgoCD (`deploy/argocd/flint-forge-applicationset.yaml`, one
`Application` per tenant) reconciles the cluster from there automatically.
The `production` GitHub Environment's required-reviewer gate still applies
to the build/push job.

### 2. Backup automation

wal-g, targeting Azure Blob Storage via Azure AD Workload Identity — no
static storage keys exist anywhere (`deploy/helm/flint-forge/templates/
postgres.yaml`, `backup-cronjob.yaml`). Tested via
`restore-drill-cronjob.yaml`, a **recurring, unattended** drill that
restores into a throwaway volume (never the live PVC) and reports
pass/fail — this satisfies "prove data comes back" continuously rather
than as a one-time human-run checkbox. Its first real result depends on a
real deployment existing and a backup cycle having run — see
`docs/runbook.md` §13.5.

### 3. Perf baselines

There is no "real staging deployment" separate from local docker-compose
in this org's infrastructure — `.github/workflows/ci.yml`'s `performance`
job now treats the `docker-compose.yml` stack as that environment, running
on every push to `main` and self-updating `perf/k6/regression.js`'s
thresholds from the measured results (`ceil(measured_p99 * 1.20)`),
committing both the thresholds and a dated file under `perf/results/`.

### 4. LLM async worker default

Decide: enable `llm.enable_background_worker` by default, or keep sync-only
as the supported default and document why (e.g. resource cost, operational
complexity) in `docs/runbook.md`.

## Verification (gate)

- A production deploy runs via CI (with human-approved gate) — not manual
  compose only. **Met**: `deploy-aks.yml` + ArgoCD, gated by the `production`
  Environment's required reviewers.
- A restore-from-backup drill is documented **and executed at least once**,
  with results recorded. **Not yet met** — the automated, recurring drill
  mechanism exists (`restore-drill-cronjob.yaml`) but has not produced a
  real result yet: nothing has been deployed to the cluster, so there is no
  backup to restore. Self-completes on the first scheduled run after
  deployment + one backup cycle; see `docs/runbook.md` §13.5.
- `perf/results/` contains committed baseline numbers; `regression.js`
  thresholds are derived from them, not placeholders. **Not yet met** — the
  automated mechanism exists and was validated by dry-run against the real
  file, but has not produced a committed result yet: it runs on push to
  `main`, and this change has not been merged yet.
- `docs/runbook.md` documents the final LLM-worker-default decision and why.
  **Met.**
