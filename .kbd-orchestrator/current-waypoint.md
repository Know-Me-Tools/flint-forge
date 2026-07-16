# Current Waypoint — Flint Forge

## Active Phase
**p16-v1.0-release-closure** — v1.0 Release Closure

## Phase State
- Status: **executing**
- Changes: 6 (1 done)
- Backend: `native-kbd` (pinned) · Delivery model: **self-hosted OSS**
- Wait budget: 2 / 3 spent

## Immediate Next Action

```
/kbd-apply p16-c002-realtime-fail-closed
```

7 tasks · no dependencies · fixes silent realtime subscription failure.

## Ordered Changes

| # | Change | Tier | Status | Depends on |
|---|---|---|---|---|
| 1 | `p16-c001-pgcron-amd64-link` | 0 | **completed** | — |
| 2 | `p16-c002-realtime-fail-closed` | 0 | pending | — (parallel with c001) |
| 3 | `p16-c003-green-integration-ci` | 0 | pending | c001 |
| 4 | `p16-c005-docs-reality-reconciliation` | 1 | pending | c002 |
| 5 | `p16-c004-security-disclosure` | 1 | **blocked** | human decision |
| 6 | `p16-c006-selfhost-operator-guide` | 1 | **blocked** | c001 + human decision |

## Why This Phase

p15 declared v1.0 production-ready. Assessment found otherwise: `v1.0.0` was
already tagged and released on 2026-07-07 (41 commits ago), and the `Postgres
integration tests` CI job has **never passed** — 0 successes across 8 runs.
Root cause is a single defect: `pg_cron` fails to link on `linux/amd64`
(`cannot find -lintl`, `images/postgres18/Dockerfile:97`). arm64 builds fine,
which is why local macOS/Colima testing never caught it — and why the published
`flint-forge-pg:18` image is **arm64-only**.

## P0 Blockers

- `pg_cron` amd64 link failure → broken image, dead integration CI (`c001`).
- `FabricChangeSource::watch()` returns `Ok(empty_stream)` → GraphQL
  subscriptions silently deliver nothing outside Helm (`c002`).
- Zero green integration runs → no integration claim is evidence-backed (`c003`).

## Wait Budget

**3 allocated; 2 spent.** One records the earlier failed local verification
attempt; one records the successful final CI observation. p15 overran at 6,
recorded as debt in its reflect handoff.
`c003` is the risk: nobody has seen that job past its build step, so failures
beyond it are unknowable. Spend waits at integration checkpoints, not on
individual functions. **If the budget reaches 3 with c003 still red: halt and
report.**

## Blocking Human Decisions

1. **Re-tag `v1.0.0` or ship `v1.0.1`?** Unresolved across assess → analyze →
   spec → plan. Blocks any release action. No release change is specced, because
   speccing one presupposes the answer.
2. Who receives a vulnerability report? (blocks `c004`)
3. Does a backup/restore runbook exist outside this repo? (blocks `c006`)

## Verification Baseline (local, 2026-07-09)

- `scripts/ci-check.sh` — green (fmt + clippy::pedantic + cargo check)
- `cargo test --workspace` with `DATABASE_URL` unset — green
- `helm lint deploy/helm/flint-forge` — green
- `scripts/check_api_versions.sh` — green
- `scripts/verify-migrations.sh` — green (11 migrations, strictly increasing)

**Not verified locally:** the `DATABASE_URL`-gated integration job and k6 runs.
Both are red or skipped in CI. See `phases/p16-v1.0-release-closure/plan.md`.
