# Goals — p16-v1.0-release-closure

## Phase Summary

Ship Flint Forge v1.0. p15 closed the production-readiness gaps; p16 turns a
verified-green workspace into a tagged, packaged, released artifact with an
operator handoff — and pays down the process debt p15 carried forward.

Seeded from: `p15-v1.0-production-readiness/reflection.md` → "Recommended Next
Phase" (P0: v1.0 release closure) and `handoffs/reflect-to-next.json`
(`next_phase_hint: v1.0-release-closure`).

---

## Inherited Debt (from p15)

Carried verbatim from `p15/handoffs/reflect-to-next.json` `open_debt`, plus one
item found during p15 closure:

1. No artifact-refiner QA logs exist for any p15 change (`.refiner/` absent).
2. `p15.total_waits` was 6, above the documented 3-wait budget.
3. k6 baselines are local Colima numbers, not production-like staging.
4. Native KBD changes were tracked in `progress.json` but never archived to
   `.kbd-orchestrator/changes/archive/<date>-<id>/`.
5. **(new)** KBD position files drifted badly out of sync during p15:
   `position.json` was 11 phases stale and `position-reminder.txt` 12 phases
   stale, causing every new session to be told it was in `p3-auth-rls-keto`.

---

## Changes (4 planned)

### P0 — Required to call it v1.0

- **p16-c001 — External CI confirmation:**
  Confirm the full GitHub Actions pipeline is green on `main`, including the
  `DATABASE_URL`-gated Postgres integration job and the Docker multi-arch image
  build. p15 verified these locally only for the DB-free subset.
  - Gate: green CI run on `main` at the release commit, linked by URL.

- **p16-c002 — Release tag and packaging:**
  Tag `v1.0.0`, generate release notes from the commit range since `v1.0.0-rc`
  (or phase start), publish the Postgres image and `forge-cli` container, and
  publish the Helm chart.
  - Gate: `v1.0.0` tag exists; artifacts resolve from their registries.

### P1 — Operator handoff

- **p16-c003 — Operator handoff docs:**
  Runbook accuracy pass against the shipped artifacts: Helm install path,
  `forge-cli migrate` / `token mint` flows, rollback procedure, and the
  monitoring panels that p15 corrected.
  - Gate: a clean-machine operator can install and reach `/healthz` from docs
    alone.

### P2 — Process debt

- **p16-c004 — KBD process hardening:**
  Address inherited debt items 1, 4, and 5. Either enforce artifact-refiner
  logs and native-change archiving, or amend the KBD contract to stop requiring
  artifacts nobody produces. Add a position-file staleness guard so the
  `p3-auth-rls-keto` drift cannot recur.
  - Gate: `/kbd-status` renders from a `position.json` no older than the
    waypoint; a phase cannot reach `completed` with unarchived native changes.

---

## Exit Condition

`v1.0.0` tagged, CI green on the release commit, artifacts published and
resolvable, operator docs validated on a clean machine, and the p15 process
debt either resolved or explicitly re-accepted in writing.

---

## Deferred

**Staging validation hardening** (p15 reflection's P1 candidate) — run k6
against production-like staging, verify Grafana panels under real traffic,
exercise the Helm install path end-to-end. Deferred because it requires
production-like infrastructure that is not currently available. Debt item 3
stays open until this runs.
