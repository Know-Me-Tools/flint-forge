# Plan — p16-v1.0-release-closure

**Phase:** 16 — v1.0 Release Closure
**Authored:** 2026-07-09
**Change backend:** native-kbd (pinned in `project.json`; see decision-log D-002)
**Changes:** 6 ordered
**Delivery model:** self-hosted OSS (decision-log D-001, provenance: user)
**Seeded from:** `assessment.md` + `analysis.md` + `handoffs/spec.json`

---

## Analyze inputs

`library-candidates.json` contains **zero candidates**, deliberately. Every gap
is a defect, a config default, a missing markdown file, or absent
infrastructure — none are library-shaped. There are therefore **no `library:`
annotations** on any change below, and no `adopt`/`adapt` reuse tasks. All six
are build changes.

One external reference is adopted as a *template*, not a dependency: the OpenSSF
Coordinated Vulnerability Disclosure guide, for `SECURITY.md` (c004).

---

## Dependency graph

```
c001 (amd64 link) ──┬──► c003 (green integration CI)
                    └──► c006 (operator guide: install path needs a pullable image)

c002 (fail closed) ─────► c005 (docs reconciliation: document the FIXED behavior)

c004 (security disclosure) ── independent, but BLOCKED on a human decision
```

Two independent roots: **c001 and c002 have no dependencies and no overlap.**
c001 touches `images/postgres18/Dockerfile`; c002 touches `crates/fdb-realtime`,
`crates/fdb-gateway`, `.env.example`. They can be worked in parallel or in
either order.

---

## Ordered change list

### 1. `p16-c001-pgcron-amd64-link` — P0, Tier 0

**Why first:** it is the taproot. It blocks c003 (the integration job's first
step is the image build) and c006 (an install guide is meaningless without a
pullable image). It is also the cheapest change in the phase — one apt package
in one builder stage.

- **Scope:** `images/postgres18/Dockerfile`, `pgcron` stage only
- **Tasks:** 5
- **Wait cost:** 0 — verified by `docker build --platform=linux/amd64`, which is
  a build, not a test-wait
- **Risk:** Low. Wrong package name fails loudly at build time.
- **Agent:** `rust-build-resolver` or direct; no domain reasoning required

### 2. `p16-c002-realtime-fail-closed` — P0, Tier 0

**Why second (or parallel with 1):** independent of c001. Highest *severity* in
the phase — the system currently reports success while delivering nothing.
Sequenced after c001 only because c001 is cheaper and unblocks more.

- **Scope:** `crates/fdb-realtime/src/lib.rs`, `crates/fdb-gateway/src/main.rs`,
  `.env.example`, `docker-compose.yml`, `CHANGELOG.md`
- **Tasks:** 7
- **Wait cost:** 1 (task t7 — integration verification; may defer to c003)
- **Risk:** Medium. **BREAKING** per Base Rule #16 — requires a CHANGELOG entry.
  Nothing that works today can break, because nothing works today.
- **Agent:** `rust-reviewer` for the `StreamError` change; `m06-error-handling`
  and `m07-concurrency` skills apply (`BoxStream`, fail-closed semantics)

### 3. `p16-c003-green-integration-ci` — P0, Tier 0

**Why third:** hard-depends on c001. This is the phase's **wait-budget risk**
and the reason the budget is allocated the way it is below.

- **Scope:** `.github/workflows/ci.yml` + whatever the job surfaces
- **Tasks:** 5
- **Wait cost:** 1–3, genuinely unknown. The job has never run past its build
  step (0 successes / 8 runs). Failures beyond the build are **unknowable until
  the image builds**. The spec deliberately does not pre-specify fixes.
- **Risk:** Unknown, and that is the finding. Budget conservatively.
- **Agent:** `build-error-resolver`

### 4. `p16-c005-docs-reality-reconciliation` — P1, Tier 1

**Why fourth:** depends on c002. Must document the *fixed* subscription
behavior, not the broken behavior — otherwise the docs are rewritten twice.

- **Scope:** `README.md`, `rest/mod.rs:62`, `.env.example`
- **Tasks:** 5
- **Wait cost:** 0 — prose; verified by `grep` and `cargo doc`
- **Risk:** Low
- **Agent:** `doc-updater`

### 5. `p16-c004-security-disclosure` — P1, Tier 1 — **BLOCKED**

**Why fifth:** no code dependency; ordered here because it is blocked on a human
decision and should not stall the Tier-0 work. Can start the moment the blocker
clears — even in parallel with c001/c002.

- **Blocked on:** *who receives a vulnerability report?* A `SECURITY.md` naming
  an unmonitored inbox is worse than none.
- **Scope:** `SECURITY.md`, `SUPPORT.md`, `CONTRIBUTING.md` (new)
- **Tasks:** 5 (t1 is the blocker)
- **Wait cost:** 0
- **Risk:** Low (documentation)
- **Agent:** `security-reviewer` to sanity-check the disclosure policy

### 6. `p16-c006-selfhost-operator-guide` — P1, Tier 1 — **BLOCKED**

**Why last:** depends on c001, blocked on a human decision, and is the largest
effort in the phase by an order of magnitude.

- **Blocked on:** *does a backup/restore runbook exist outside this repo?* If
  yes, this collapses to a doc link plus verification. If no, it is days of
  genuine engineering. **Answer before estimating.**
- **Scope:** `docs/operations/backup-restore.md`, `docs/operations/upgrading.md`
- **Tasks:** 7
- **Wait cost:** 1 (task t7 — clean-machine end-to-end install/backup/restore)
- **Risk:** High effort, high value. The `flint_vault` KMS-wrapped DEK is **not
  in a `pg_dump`**; a restore without it yields unreadable secrets. Criterion 1
  demands the restore be *executed*, not theorized — that is where this will be
  discovered wrong.
- **Agent:** `devops-engineer` + `database-reviewer`

---

## Wait budget

**Allocated: 3.** This is the documented per-epoch budget (Base Rule: Integration-First,
`docs/RUST-DEVELOPMENT-MANAGEMENT.md`). p15 overran it at **6**, which
`p15/handoffs/reflect.json` records as debt.

| Change | Waits | Spent on |
|---|---|---|
| c001 | 0 | `docker build` is a build, not a test |
| c002 | 1 | integration verification of the default path (may fold into c003) |
| c003 | 1–3 | **the risk** — unknown failures past the build step |
| c005 | 0 | prose; `grep` + `cargo doc` |
| c004 | 0 | prose |
| c006 | 1 | clean-machine restore drill |

**Allocation strategy:** spend waits at genuine integration checkpoints, not on
validating individual functions as they are written. Concretely: do **not** wait
for c002's integration test in isolation — land c001, land c002, then take one
wait on c003 that validates the image, the migrations, and the subscription path
together. If c003 needs a second and third wait, that is the budget consumed and
the phase should stop and report rather than silently overrun as p15 did.

**Escalation:** if the budget reaches 3 with c003 still red, halt and surface it.
Do not continue burning waits.

---

## Parallelism

- **c001 ∥ c002** — no shared files, no dependency. Run concurrently if two
  agents are available.
- **c004** — unblocked by neither; can start the instant its human decision
  lands, at any point in the phase.
- Everything else is strictly sequential on its dependency.

---

## Exit condition

1. `Postgres integration tests` reports `success` on `main`, **twice** (c003).
2. `docker manifest inspect` lists both `linux/amd64` and `linux/arm64` (c001).
3. No `Ok(futures::stream::empty())` on any `ChangeStreamSource::watch`; the
   default configuration delivers a real subscription event (c002).
4. `SECURITY.md` names a monitored channel (c004).
5. Docs describe only behavior that a default install actually exhibits (c005).
6. A clean-machine operator can install → backup → destroy → restore →
   `/healthz` from docs alone (c006).

**Not in the exit condition:** the `v1.0.0` re-tag. See below.

---

## Deferred / out of scope

Per the self-hosted OSS delivery model (D-001):

- status page, on-call rotation, incident comms
- DPA / ToS / privacy posture, SLA, support response targets
- k6 load validation — blocked on staging infrastructure that does not exist
  (`analysis.md` Finding 5; `ci.yml:98` gates it behind `workflow_dispatch`)
- CVE numbering authority, SBOM
- automated backup tooling, HA / replication / failover

---

## Open questions — unresolved across three stages

These have now survived assess → analyze → spec → plan. The first blocks any
release action and **cannot be answered by an agent**:

1. **Re-tag `v1.0.0` or ship `v1.0.1`?** `v1.0.0` is public (tag `310e7f6`,
   released 2026-07-07) with an arm64-only image, and 41 commits have landed
   since. Retract-and-retag preserves the number but rewrites a published
   release; `v1.0.1` is honest but concedes v1.0.0 shipped broken. **Human
   decision.** This is why no release change appears in the ordered list above —
   speccing one would presuppose the answer.
2. **Who receives a vulnerability report?** (blocks c004)
3. **Does a backup runbook exist outside this repo?** (blocks c006)
4. **Does the beta include realtime subscriptions?** c002 assumes yes. If no,
   the honest move is to remove them from `README.md`, not repair them.
5. **Is `v0.10.0 → v1.0.0` a supported upgrade?** p35-c004 landed a `BREAKING`
   change to `PgBackend::acquire` RLS setup. May require manual steps (c006 t6).

---

## First change to apply

```
/kbd-apply p16-c001-pgcron-amd64-link
```

5 tasks. No dependencies. No wait cost. Unblocks half the phase.
