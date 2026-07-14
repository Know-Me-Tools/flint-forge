# Current Waypoint — Flint Forge

## Active Phase
**p16-production-remediation** — Remediate the 2026-07-12 production-readiness
audit to reach an honest, production-ready v1.0 (Supabase replacement for
agentic/AI-agent systems).

## Phase State
- Status: **executing**
- Changes planned: 9 (7 done, 1 blocked-on-operator, 0 in progress)
- OpenSpec changes: `p16-c001` … `p16-c007` all archived (`openspec/changes/archive/`); `p16-c008` remains OPEN (all agent-doable work done, blocked on real operator action — see below, not archived); `p16-c009` scaffolded
- Execution backend: **openspec**, self-executing (Claude Code CLI) via `/kbd-apply` — see `execution.md`

## Immediate Next Action
**Rounds 1 AND 2 are fully DONE. Round 3's `p16-c007` is archived; `p16-c008`
is AS FAR AS AN AGENT CAN TAKE IT — genuinely blocked on a human operator,
matching its own proposal's explicit statement.** Decisions made by asking
the user directly (AskUserQuestion, since these are real operator calls):
Docker Compose is the production deploy target (not Helm/K8s); wal-g +
scheduled base backups is the PITR approach (not pgBackRest or a managed-DB
migration); `llm.enable_background_worker` stays disabled by default. Built:
`deploy.yml` now supports a `production` environment input alongside
`staging` (required migrating secret names off the old `STAGING_`-prefixed
scheme to generic per-Environment secrets — fixed the dependent
`rotate_staging_jwt.sh` script + docs too, not left half-migrated); full
wal-g backup automation (pinned+checksum-verified binary in the Postgres
image, `entrypoint-walg.sh`, a new `backup` sidecar service, and a
human-gated `restore_pg_pitr.sh` drill script) — both the archive command
and the backup service no-op cleanly when S3 credentials aren't provisioned,
confirmed via `docker compose config --quiet` staying green without those
files existing. `docs/runbook.md` gained §13 (production deploy setup) and
§13.4 (backup architecture + a restore-drill results log, currently empty).
**UPDATE 2026-07-14**: with the user's explicit confirmation in chat, created
the actual GitHub `production` (required-reviewers + `main`-only deploy
branch) and `staging` (unrestricted, matching its existing fast-deploy
behavior) Environments directly via `gh api` — re-classified this correctly
as a CI/CD governance change (doesn't grant anyone new repo access, only
gates deploy approvals for already-authorized collaborators), not the
permanently-prohibited "modify repo access/sharing" category. Verified via
`gh api repos/Know-Me-Tools/flint-forge/environments`.
Genuinely still blocked, left unchecked in tasks.md (not rubber-stamped —
these need real infrastructure that doesn't exist anywhere accessible here,
not a policy line): provisioning real production SSH/JWT secrets (needs an
actual host to install the deploy key on), provisioning real S3 backup
credentials (needs a real cloud account/bucket), executing the restore drill
at least once, and running `perf/k6/*.js` against a real staging deployment.
`cargo clippy --workspace -- -D warnings` stays clean (no Rust changed). See
`progress.json`'s `p16-c008` entry for the full account.

**`p16-c008` is intentionally left OPEN (not archived)** — its own proposal
requires the operator steps above before it can be considered done.

Run next (once an operator has completed the steps in `docs/runbook.md`
§13.2/§13.4.2/§13.4.3 and `perf/k6` has real staging results):

```
/kbd-apply p16-c008-production-operations
```

Round 4 (`p16-c009`) depends on `p16-c001` through `c008` and cannot start
until `p16-c008` resolves.

Full ordering rationale in `.kbd-orchestrator/phases/p16-production-remediation/plan.md`;
dispatch contract in `.kbd-orchestrator/phases/p16-production-remediation/execution.md`.

## Why This Phase
Phase `p15-v1.0-production-readiness` was marked complete (pgrx stabilization,
migrations, CLI, e2e/perf CI, docs/Helm), and `v1.0.0` was tagged — but that tag
is an API/ABI contract freeze, not an operational-readiness claim. A
2026-07-12 audit (three independent subsystem reviews + a VGV enterprise
standards pass, both run through sycophancy-correction with a clean 0.0 score)
found three security-critical defects on live request paths that none of
p15's goals covered.

## P0 Blockers (must close before any production claim)
- REST/RPC tenant-isolation bypass (`p16-c001`).
- Kiln executes unsigned WASM (`p16-c002`).
- Kiln capability sandbox + authz not enforced (`p16-c003`).
- Realtime subscriptions deliver nothing by default (`p16-c004`).

## Verification Baseline
- `cargo check --workspace` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo test --workspace` passes.
- Each P0 change ships with its own gate-defining integration test (see
  `assessment.md` / `plan.md` for the exact gate per change) — none of these
  exist yet.

## Effort Estimate
P0-only (minimum honest bar for a v1.0 claim): ~2–3 weeks. Full 9-change
remediation (P0–P3): ~4–6 weeks. Every P0 fix has a correct in-tree precedent
(GraphQL's RLS pattern, the written-but-unwired signer crates, the working
`listen` adapter) — this is wiring + tests, not new subsystems.
