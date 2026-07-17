# p16-c001 — pg_cron amd64 link fix

**Phase:** p16-v1.0-release-closure
**Priority:** P0 — Tier 0, ship-blocker
**Scope:** `images/postgres18/Dockerfile` (pgcron builder stage only)
**Delivery model:** model-independent (required for all)

---

## Problem

`pg_cron` fails to link on `linux/amd64`:

```
/usr/bin/ld: cannot find -lintl: No such file or directory
collect2: error: ld returned 1 exit status
make: *** [.../Makefile.shlib:261: pg_cron.so] Error 1
```

Failing command is `images/postgres18/Dockerfile:97`. The `pgcron` builder stage
(`FROM rust:1.96-bookworm AS pgcron`, line 85) installs `postgresql-common`,
`gnupg`, `ca-certificates`, `git`, `make`, `gcc`, and `postgresql-server-dev-18`
— but never `gettext` / `libintl`. `pg_cron`'s link line ends `-lpq -lintl`.

**Why it went unnoticed:** the `arm64` build succeeds. Local development is
macOS/Colima (arm64), so `docker build` passes on every developer machine and in
the arm64 CI leg. This is a pure architecture asymmetry.

## Blast radius

- `Postgres integration tests` CI job: **0 successes across 8 recent runs**.
- `Postgres image` workflow: `Build and push linux/amd64` fails →
  `Create multi-arch manifest` is **skipped** → published `flint-forge-pg:18`
  is **arm64-only**.
- Consequence: v1.0.0 shipped an image that no x86_64 operator can pull. For a
  self-hosted OSS product this is the single most damaging defect present.

## Change

**CORRECTED 2026-07-10 during t1 — the original "add a package" premise was
wrong.** Empirically verified against `rust:1.96-bookworm` on `linux/amd64`:

- `-lintl` is **not** emitted by Postgres. `pg_config --libs` does not contain
  it. It comes from `pg_cron`'s own `Makefile:22`:
  `SHLIB_LINK = $(libpq) -lintl`.
- On Debian bookworm glibc, `libintl` functionality is **built into glibc**.
  There is **no `libintl.so` link target at all** — not from `gettext`, not
  from `libgettextpo-dev`, not from `libc6-dev`. A bare `main()` links cleanly
  with **no** `-lintl`. Adding any package does **not** create the target.
- arm64 succeeds because its base layout differs; amd64/glibc has no such lib.

So no apt package can fix this. The `-lintl` flag is **spurious on glibc** and
must be **removed** from pg_cron's link line. In the `pgcron` builder stage,
after `git clone`, strip it:

```dockerfile
RUN sed -i 's/^SHLIB_LINK = $(libpq) -lintl/SHLIB_LINK = $(libpq)/' Makefile
```

(before the `make` step, `images/postgres18/Dockerfile:97`). This is the
minimal, correct fix. Touch only the `pgcron` stage.

**Alternatives considered and rejected:**
- Add `gettext` / `libgettextpo-dev` — verified NOT to provide the link target.
- `ln -s` a runtime `libintl.so.*` to `libintl.so` — no runtime lib exists to
  link against on this base.
- `SHLIB_LINK` override via `make SHLIB_LINK=…` — the `sed` is clearer and
  survives pg_cron's own Makefile include order.

## Acceptance Criteria

1. `docker build --platform=linux/amd64 -f images/postgres18/Dockerfile .`
   completes successfully on a machine that is **not** arm64-native (or under
   emulation), producing `pg_cron.so`.
2. `docker build --platform=linux/arm64 …` still succeeds (no regression).
3. The `Postgres image` workflow's `Build and push linux/amd64` job reaches
   `success`, and `Create multi-arch manifest` runs rather than skipping.
4. `docker manifest inspect ghcr.io/<owner>/flint-forge-pg:18` lists **both**
   `linux/amd64` and `linux/arm64`.

## Non-Goals

- Fixing the `Postgres integration tests` job end-to-end (that is p16-c003;
  this change only removes its first blocker).
- Upgrading `pg_cron`, changing the Postgres base image, or touching the
  `anvil` / `pgnet` builder stages.

## Verification Command

```bash
docker build --platform=linux/amd64 -t flint-forge-pg:18-amd64 \
  -f images/postgres18/Dockerfile .
```

Must exit 0. This is the whole gate — if it passes, the CI job's build step
passes.

## Risk

**Low.** Additive apt package in one builder stage. The failure mode if the
wrong package name is chosen is a build error, caught immediately, not a silent
runtime defect.

## Open Questions

- Is amd64 actually required? (carried from `assessment.md`) If every deployment
  target were arm64, the alternative fix is to drop the multi-arch manifest
  rather than repair the linker. **For self-hosted OSS the answer is yes** —
  operators run on hardware we do not control, and x86_64 dominates. This
  change proceeds on that basis.
