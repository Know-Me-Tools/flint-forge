# Reflection — p13-continuous-operations

**Phase:** 13 — Continuous Operations
**Period:** 2026-07-06
**Author:** OpenCode / KBD automated reflection
**Changes:** 4/5 done (1 deferred)
**Status:** ✅ COMPLETE — p14 exit condition met

---

## Summary

Phase 13 was the post-release steady-state phase. It delivered four standing
changes: an API versioning CI gate, a monthly dependency maintenance pass, a
monitoring reference with threshold review schedule, and a v1.1.0 roadmap with
6 prioritised items. The deferred change (k6 baselines) remains blocked on a
staging host and is carried forward as the P0 item in `docs/ROADMAP.md`.

The dependency maintenance pass (c002) discovered a transitive resolution
conflict: `generic-array 0.14.7→0.14.9` pulls `sqlx 0.9.0` into pgvector's
dependency tree, which breaks `Encode`/`Type` trait resolution across the
sqlx 0.8/0.9 boundary. 17 safe patch bumps were applied; `generic-array` was
excluded with a documented note.

---

## Goal Achievement

| Goal | Priority | Status | Evidence |
|---|---|---|---|
| G1 — k6 baselines | P0 | **DEFERRED** | Staging unavailable; P0 carry-forward in `docs/ROADMAP.md` |
| G2 — dependency maintenance | P1 | **MET** | 17 safe bumps; `cargo audit` clean; `generic-array` excluded with rationale |
| G3 — monitoring review | P1 | **MET** | `docs/monitoring.md` — alert rules, Grafana guide, 30d/quarterly review schedule, known limitations |
| G4 — API versioning gate | P1 | **MET** | `docs/api/versioning.md` (policy); `scripts/check_api_versions.sh` (CI gate); `ci.yml` step added |
| G5 — v1.1.0 planning | P2 | **MET** | `docs/ROADMAP.md` with 6 items; p14 exit condition met |

**Phase exit condition:** `ROADMAP.md` has 6 items (≥ 3 required) → p13 transitions to p14.

---

## Deliverables

| File | Lines | Purpose |
|---|---|---|
| `docs/api/versioning.md` | ~130 | Breaking change policy, checklist tables, deprecation process |
| `scripts/check_api_versions.sh` | ~80 | Stateless CI gate; parses version from docs + `.env.example` |
| `.github/workflows/ci.yml` | +6 lines | `API version check` step after `Security audit` |
| `docs/monitoring.md` | ~110 | Alert rules reference, Grafana guide, review schedule, webhook setup |
| `docs/ROADMAP.md` | ~80 | 6 prioritised items for v1.1.0 |
| `Cargo.lock` | 17 bumps | Monthly dependency maintenance (cc, crossbeam-*, pest*, quinn-*, etc.) |

---

## Technical Debt

| Item | Source | Severity | Resolution |
|---|---|---|---|
| k6 baselines still TBD | p13-c001 | LOW | Run k6 against staging (operator action) |
| `generic-array` excluded from `cargo update` | p13-c002 | LOW | Upgrade workspace to sqlx 0.9 in a future phase (ROADMAP P1 item) |
| Grafana DB connections panel shows "no data" | p10-c004 | LOW | sqlx Prometheus integration (ROADMAP P1 item) |
| Alertmanager webhook not configured | p10-c004 | LOW | Operator sets `ALERTMANAGER_WEBHOOK_URL` in `.env` |

**No new architectural debt introduced in p13.** All items are carry-forward from p9–p12 or external dependencies.

---

## What Was Harder Than Expected

1. **`generic-array` transitive resolution conflict** — A seemingly innocuous `0.14.7→0.14.9` patch bump pulled `sqlx 0.9.0` into pgvector's dependency tree. The `Encode`/`Type` traits from sqlx 0.8 and 0.9 don't unify, causing a compile error in `fdb-reflection`. The fix was to exclude `generic-array` from the update batch and document the conflict for the sqlx 0.9 upgrade (a ROADMAP item).

2. **Stale build artifacts after Cargo.lock revert** — After reverting the Cargo.lock to fix the generic-array issue, `cargo check` failed with a proc-macro parse error (`async-graphql-derive`). This was caused by stale artifacts from the sqlx 0.9 compilation attempt. `cargo clean` resolved it, but required a full rebuild (~6 min).

---

## Lessons Captured

1. **`cargo update` can pull in major version bumps for transitive dependencies** — Even when workspace deps are pinned to `0.8`, a transitive dependency can resolve to `0.9` if its version range allows it. Always run `cargo test` after `cargo update`, and if a failure occurs, check the Cargo.lock diff for unexpected major version changes in transitive deps.

2. **`cargo clean` is the first fix for stale-artifact proc-macro errors** — When `cargo check` fails with "failed to parse process output" for a proc-macro after a Cargo.lock revert, the build cache has stale artifacts from the previous resolution. `cargo clean` + rebuild resolves it.

3. **Stateless CI gates are better than git-diff-based ones** — `check_api_versions.sh` parses version integers from files and compares them. No git history needed. This makes the check deterministic across all environments (local, CI, worktrees) and immune to merge/rebase edge cases.

4. **A continuous-operations phase can produce real deliverables** — p13 was framed as "standing mode" but delivered four concrete changes: a CI gate, a maintenance pass, a monitoring doc, and a roadmap. The event-triggered model works — each change had a clear trigger, and the phase ended naturally when the roadmap exit condition was met.

---

## Recommended Next Phase

**p14-v1.1.0** — the first feature cycle after v1.0.0. Seeded from `docs/ROADMAP.md`:

| Priority | Item | Scope |
|---|---|---|
| P1 | sqlx Prometheus integration | Medium — fixes DB connections panel |
| P1 | Kiln guest Rust SDK (`flint-skill`) | Medium — new crate for skill authors |
| P1 | A2UI component hot-reload | Medium — extends StateManager |
| P2 | Staging JWT rotation automation | Small — shell script + CI |
| P2 | Kiln per-function invocation metrics | Small — 3 counters |

**Estimated scope:** 5 changes, 2–3 sessions. The sqlx 0.9 upgrade (needed for the Prometheus integration) is the largest single item and should be assessed first.

---

*Generated by OpenCode `/kbd-reflect` — 2026-07-06*
