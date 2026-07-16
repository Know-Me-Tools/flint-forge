# p16-c003 — One green Postgres integration run

**Phase:** p16-v1.0-release-closure
**Priority:** P0 — Tier 0, ship-blocker
**Scope:** `.github/workflows/ci.yml`, whatever the failing job surfaces
**Depends on:** p16-c001 (amd64 link fix)
**Delivery model:** model-independent

---

## Problem

The `Postgres integration tests` job has **never passed**. Eight consecutive
runs on `main`: seven `failure`, one `cancelled`. Zero successes, ever.

Runs inspected: 29056242160, 29046846501, 29044804997, 29041847319,
29036822529, 29035641785 (cancelled), 29035267504, 29033153874.

Its first step is `docker build -f images/postgres18/Dockerfile .` — which fails
at the `pg_cron` amd64 link (p16-c001). That is the *first* blocker. Whether
anything fails *after* the image builds is **unknown**, because no run has ever
reached the test step.

## Why this matters

Every integration claim in every document is currently unbacked by evidence.
p15's reflection graded G4 ("E2E and performance validation") as MET. It was
not. p15's `progress.json` claimed the job was "green in CI" — it never was.
Until this job succeeds once, no statement about integration coverage is
anything but assertion.

## Change

1. Land p16-c001. Re-run CI. Observe how far the job gets.
2. Fix whatever the job surfaces *after* the image builds. This is
   deliberately open-ended: **the failures beyond the build step are not yet
   knowable.** Do not pre-specify fixes for problems that may not exist.
3. Iterate until the job reports `success` once on `main`.

## Acceptance Criteria

1. `gh run list --workflow=ci.yml --branch main --limit 1` shows the
   `Postgres integration tests` job with conclusion `success`.
2. The run URL is recorded in this change's `verification.md`.
3. The success is reproducible: a second push also yields `success` (not a
   flake).

## Non-Goals

- k6 / performance gating. `ci.yml:98` gates the k6 job behind
  `workflow_dispatch` and a `STAGING_BASE_URL` secret that no environment
  supplies. Out of scope; see `analysis.md` Finding 5.
- Making the job fast.

## Verification Command

```bash
gh run list --workflow=ci.yml --branch main --limit 3 \
  --json databaseId,conclusion,headSha
gh run view <id> --json jobs \
  -q '.jobs[] | select(.name=="Postgres integration tests") | .conclusion'
```

Must print `success`.

## Risk

**Unknown, and that is the point.** Nobody has seen this job past its build
step. It may pass immediately once the image builds; it may surface a cascade of
DB-gated test failures. Estimate honestly: **1 test-wait minimum, possibly
several.** Budget accordingly (p15 overran its 3-wait budget at 6).

## Open Questions

- Does the DB-gated test suite actually pass? `scripts/ci-test.sh` runs it, and
  p15's notes claim local success, but "local" meant arm64 with a hand-built
  image. Unverified on CI's amd64.
