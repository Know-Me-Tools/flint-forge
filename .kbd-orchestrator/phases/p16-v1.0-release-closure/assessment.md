# Assessment — p16-v1.0-release-closure

**Phase:** 16 — v1.0 Release Closure
**Assessed:** 2026-07-09
**Assessor:** claude-code
**Method:** git/gh inspection of `main`, CI run history, Dockerfile source, workflow definitions

---

## Headline

**p16's premise is wrong, and p15 was closed on a false claim.**

Two facts discovered during assessment invalidate the phase as planned:

1. **`v1.0.0` is already tagged and released.** Tag `310e7f6` (2026-07-07),
   GitHub Release "Flint Forge v1.0.0 — First Stable API Release" published
   2026-07-07T12:03:23Z. p16-c002 ("tag v1.0.0") is not pending work — it
   happened two days ago, and **41 commits have landed on `main` since**.

2. **CI has never been green on `main`.** The `Postgres integration tests` job
   — the p15-c004 deliverable — has **failed or been cancelled on all 8 of the
   last 8 runs**. It has no recorded success, ever.

p15 was marked `completed` (by me, 2026-07-09) on the strength of five *local*
gates. Those gates were real and did pass, but none of them exercise the job
that fails. The `closure_note` I wrote explicitly flagged this ("NOT verified
locally: the DATABASE_URL-gated integration job … confirmed present and green
in CI only"). That parenthetical was **false**: it was never green in CI. I
carried forward an unverified claim from `progress.json`'s c004 notes
("Integration run is now green except for final CI confirmation") instead of
checking `gh run list`. This assessment corrects the record.

---

## Root Cause: `pg_cron` amd64 link failure

```
/usr/bin/ld: cannot find -lintl: No such file or directory
collect2: error: ld returned 1 exit status
make: *** [.../Makefile.shlib:261: pg_cron.so] Error 1
```

- **Location:** `images/postgres18/Dockerfile:97` (`pgcron` builder stage).
- **Trigger:** `pg_cron`'s link line ends `-lpq -lintl`. The builder stage
  installs `postgresql-server-dev-18`, `git`, `make`, `gcc` — but never
  `libgettextpo-dev` / `gettext` , so `libintl` is absent.
- **Why it hid:** the **arm64** build succeeds; only **amd64** fails. Local dev
  is macOS/Colima (arm64), so `docker build` passes on every developer machine
  and in the arm64 CI leg. This is a pure architecture asymmetry.

**Blast radius beyond CI:** the same failure breaks the `Postgres image`
workflow's `Build and push linux/amd64` job, which causes `Create multi-arch
manifest` to be **skipped**. So the published `flint-forge-pg:18` image is
**arm64-only**. Any operator on x86_64 — i.e. essentially all cloud
deployments — cannot pull a working image. This is a release-blocking defect in
a *released* v1.0.0.

---

## Goal-by-Goal Assessment

| Change | Planned Goal | Status | Evidence |
|---|---|---|---|
| p16-c001 | External CI confirmation green on `main` | **NOT MET — blocked** | `Postgres integration tests` failed on runs 29056242160, 29046846501, 29044804997, 29041847319, 29036822529, 29035267504, 29033153874; cancelled on 29035641785. Zero successes. |
| p16-c002 | Tag `v1.0.0`, publish artifacts | **PARTIAL / already done, incorrectly** | Tag + Release exist since 2026-07-07. But the image is arm64-only (multi-arch manifest skipped), so the release's headline artifact is broken on amd64. 41 commits on `main` postdate the tag. |
| p16-c003 | Operator handoff docs | **NOT ASSESSED** | Cannot validate a clean-machine install path while the amd64 image does not exist. Blocked behind c001/c002. |
| p16-c004 | KBD process hardening | **NOT MET — and now higher priority** | Inherited debt confirmed: `.refiner/` absent (no QA logs, any phase); `.kbd-orchestrator/changes/archive/` absent. New evidence: the closure process let a "verified" phase ship on an unchecked CI claim. |

---

## Additional Findings

- **k6 was never a gate.** `ci.yml:98` — `if: github.event_name ==
  'workflow_dispatch'`. The `k6 Performance Regression` job is **skipped on
  every push** and requires a `STAGING_BASE_URL` secret. p15's "k6 baselines"
  deliverable is four script files (`perf/k6/{health,regression,components,
  mcp_tools}.js`), not an enforced threshold. `reflect.json` debt item 3
  ("local Colima baselines") understates this: the baselines are not measured
  against anything in CI at all.

- **`Rust checks` is genuinely green.** The one job that consistently passes is
  the fmt/clippy/check leg — which is precisely what `scripts/ci-check.sh` runs
  locally. Local green ⇏ CI green, because local never builds the image.

- **Working tree is dirty** with the p15 closure edits (`current-waypoint.json`,
  `plan.md`, and the untracked p16 directory). Uncommitted.

- **Version coherence holds:** workspace `Cargo.toml` `1.0.0`, Helm
  `Chart.yaml` `version: 1.0.0` / `appVersion: "1.0.0"`, tag `v1.0.0`. Nothing
  to reconcile there.

---

## Gaps for Plan Stage

Ordered by what blocks what:

1. **Fix `pg_cron` amd64 link** (`images/postgres18/Dockerfile`, pgcron builder
   stage): add `gettext` / `libgettextpo-dev` (or drop `-lintl` via a `pg_cron`
   build flag). Verify by building `--platform=linux/amd64` locally, not just
   the native arm64. **This is the single blocker for c001, c002, and c003.**

2. **Get `Postgres integration tests` green once.** Until it has one recorded
   success, no statement about integration coverage is evidence-backed.

3. **Re-cut the release.** Options: `v1.0.1` with the amd64 fix, or retract and
   re-tag `v1.0.0`. Must produce a working multi-arch manifest. Decide whether
   the 41 post-tag commits belong in the release.

4. **Decide k6's status.** Either wire it as a real gate (needs staging +
   secret) or stop describing it as a validation deliverable.

5. **Process hardening (c004), promoted from P2.** The failure mode here was
   not technical — it was a closure that trusted a note instead of a check. At
   minimum: a phase must not reach `completed` while its own CI job has zero
   successes.

---

## Open Questions for Plan

- **Re-tag or patch-release?** `v1.0.0` is public with a broken amd64 image.
  Retract-and-retag preserves the version number but rewrites a published
  release; `v1.0.1` is honest but concedes v1.0.0 shipped broken. **Needs a
  human decision.**
- **Do the 41 post-`v1.0.0` commits ship in the next tag,** or should the fix
  be cherry-picked onto the tag?
- **Should p15 be reopened** rather than left `completed`? Its c004 goal
  ("CI database integration") is objectively not met. Reflecting reality argues
  for reopening; process-wise, p16-c001 already tracks the same work.
- **Is amd64 actually required?** If every deployment target is arm64, the
  multi-arch manifest could be dropped instead of fixed — but README and Helm
  make no such restriction.

---

## Recommendation

**Do not proceed to release packaging.** Reorder p16 to put the amd64 Dockerfile
fix first as a new P0 change, then re-verify CI, then revisit tagging. The
phase as written assumes a green pipeline and an untagged release; neither
holds.
