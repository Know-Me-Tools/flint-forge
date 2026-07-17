# p16-c008 Tasks — Production Operations

## Tasks

**2026-07-14 architecture correction:** the original plan assumed a single
SSH-reachable production host, S3-compatible backup storage, and a separate
"staging" environment worth deploying to and testing against — none of which
match this org's real infrastructure (a shared, multi-tenant AKS cluster on
Azure with ArgoCD already running in-cluster; no staging host exists,
short of a real client deployment). The SSH/S3 automation from the earlier
pass was fully replaced with the Kubernetes/Azure equivalent below —
described in `docs/runbook.md` §13.

- [x] Clarify with operator: compose or Helm/K8s is the canonical production
      deploy target — CORRECTED 2026-07-14: Kubernetes (existing shared AKS
      cluster `main`, resource group `prometheus-rg`), via the existing
      in-cluster ArgoCD instance, not a new SSH-managed host.
- [x] Retire `.github/workflows/deploy.yml` (SSH/compose, single-host —
      doesn't match reality) and add `.github/workflows/deploy-aks.yml`
      (builds + pushes images to the shared ACR `prometheusagsacr.azurecr.io`
      via GitHub OIDC, then commits the new tag into
      `deploy/helm/flint-forge/values-<tenant>.yaml` — that commit is the
      deploy; ArgoCD reconciles from there, no SSH/kubectl from CI at all).
- [x] Add GitHub Environment protection rules (required reviewers) for
      production deploys — done 2026-07-14 (see prior entry in git history);
      still gates `deploy-aks.yml`'s `build-and-push` job. Added a second,
      environment-scoped OIDC federated credential
      (`repo:Know-Me-Tools/flint-forge:environment:production`) so the
      approval-gated job can still authenticate to Azure.
- [x] Provision production deploy credentials/secrets — SUPERSEDED, not
      applicable: `deploy-aks.yml` authenticates via GitHub OIDC federated to
      the existing shared `github-actions-aks-deploy` Azure AD app (already
      used by 6 other projects on this cluster) — there is no SSH key or
      static credential to provision at all. Repo Variables
      `AZURE_CLIENT_ID`/`AZURE_TENANT_ID`/`AZURE_SUBSCRIPTION_ID` are set
      (non-secret identifiers, not credentials — OIDC has no shared secret).
- [x] Choose backup approach: `pgBackRest`/`wal-g` sidecar vs.
      managed-Postgres migration — wal-g (unchanged).
- [x] Implement the chosen automated backup job with PITR — REWORKED
      2026-07-14 from S3 to Azure Blob Storage via Azure AD Workload
      Identity (`deploy/helm/flint-forge/templates/postgres.yaml`,
      `backup-cronjob.yaml`) — zero static storage keys anywhere.
- [x] Provision backup storage target credentials — SUPERSEDED, not
      applicable: real Azure resources were provisioned directly (resource
      group `flint-forge-rg`, storage account `stflintforgebakc69689`,
      container `pg-backups`, managed identity `flint-forge-walg-identity`
      federated to the AKS cluster's OIDC issuer, `Storage Blob Data
      Contributor` scoped to the container). Workload Identity means there
      is no key/secret to provision at all — see `docs/runbook.md` §13.2/13.4.
- [x] Automate the restore drill so it no longer requires an operator to run
      it — `deploy/helm/flint-forge/templates/restore-drill-cronjob.yaml`
      (weekly CronJob, restores into a throwaway `emptyDir`, never the live
      PVC, so it's safe to run unattended on a schedule).
- [ ] Execute and document at least one real restore-drill result in
      `docs/runbook.md` §13.5 — NOT done: the automation exists and is
      wired to run itself, but flint-forge has not been deployed yet (no
      Application has been applied to the cluster, no data exists, no
      backup has run) — there is genuinely nothing to restore yet. This will
      self-complete on the first scheduled run after deployment + one backup
      cycle; the results table documents this explicitly rather than being
      rubber-stamped.
- [x] Automate `perf/k6/*.js` so it no longer requires a real staging
      deployment — `.github/workflows/ci.yml`'s `performance` job now runs
      on every push to `main`, against the `docker-compose.yml` stack
      (the only "staging" that exists — see `docs/runbook.md` §13.1), and
      self-updates its own thresholds from the measured results.
- [x] Commit k6 results to `perf/results/` — automated as part of the same
      CI job (writes `perf/results/<date>-docker-compose-ci.json` and
      commits it); will produce its first real file on the next push to
      `main` that runs this job.
- [x] Update `perf/k6/regression.js` thresholds to `measured_p99 * 1.20` —
      automated via the same job's `handleSummary()` → `perf-summary.json` →
      Python threshold-rewrite step (dry-run validated against the real file
      during this change — see commit history).
- [x] Decide `llm.enable_background_worker` default (on vs. documented-sync-only)
- [x] Document the LLM-worker-default decision and rationale in `docs/runbook.md`
- [x] Update `docker-compose.prod.yml`'s `db` comment — REFRAMED: production
      is no longer compose-based at all, so the "staging only" caveat is
      replaced with a pointer to the Kubernetes path as the supported
      target; the compose S3 backup mechanism remains as an unsupported
      fallback for anyone running a standalone single-host instance.
- [x] `cargo clippy --workspace -- -D warnings` clean — unaffected: no Rust
      source changed in this pass (Helm/YAML/shell/CI only).

## Remaining before this change can be archived

1. Apply `deploy/argocd/flint-forge-applicationset.yaml` to the `main`
   cluster (`kubectl apply -f ...`) — not yet applied; needs a final
   go/no-go since it starts consuming resources on a shared cluster other
   teams also use.
2. Push this branch's commits to `main` — `deploy-aks.yml` and the CI
   `performance` job only start running (and thus only start producing the
   real restore-drill/k6-baseline results the two items above are waiting
   on) once this lands on `main`.
