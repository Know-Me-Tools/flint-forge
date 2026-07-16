# Verification — p16-c001

## Gate
`docker build --platform=linux/amd64 -f images/postgres18/Dockerfile .` exits 0.

## Evidence to record on completion
- [x] amd64 full image build exits 0; pgcron stage produces and installs `pg_cron.so`
- [x] arm64 full image build exits 0 (no regression)
- [x] `Build and push linux/amd64` job concludes `success`
- [x] `Create multi-arch manifest` runs and concludes `success`

## Final evidence (2026-07-10)

- Implementation commit: `f18eb33c75dcb7a711bea3800618e1263d382ef9`
- Local amd64 image: `flint-forge-pg:p16-c001-amd64`, build exit 0
- Local arm64 image: `flint-forge-pg:p16-c001-arm64`, build exit 0
- GitHub Actions run: https://github.com/Know-Me-Tools/flint-forge/actions/runs/29122248786
  - `Build and push linux/amd64`: success (12m42s)
  - `Build and push linux/arm64`: success (1m36s)
  - `Create multi-arch manifest`: success (26s)
- Native KBD verification: `verify: PASS` with 5/5 tasks complete

## t1 finding (2026-07-10) — root cause is NOT a missing package

Empirically verified against `rust:1.96-bookworm`:
- `-lintl` originates in pg_cron `Makefile:22` (`SHLIB_LINK = $(libpq) -lintl`),
  NOT from Postgres (`pg_config --libs` has no `-lintl`).
- On bookworm glibc there is NO `libintl.so` link target on **either** arch —
  `gettext`, `libgettextpo-dev`, `libc6-dev` all fail to provide it. glibc
  supplies intl internally; a bare `main()` links with no `-lintl`.
- FIX: strip `-lintl` from SHLIB_LINK via `sed` in the pgcron builder stage.
- PROOF: with `-lintl` removed, `make` produces `pg_cron.so` on amd64 (EXIT=0).
- SAFE on arm64: `-lintl` also fails to resolve there, and a bare link succeeds,
  so removal cannot regress arm64.

The original spec's "add a package" approach is disproven and was corrected.

## t3 finding (2026-07-10) — local amd64 verification is BLOCKED by QEMU, not by the fix

Full `docker build --platform=linux/amd64` on this arm64 host fails, but **not**
at the pg_cron stage — the pg_cron fix worked and that stage is now past. It
fails **earlier**, at `Dockerfile:33` (`anvil` stage,
`cargo install cargo-pgrx --version 0.18.1`):

```
error: linking with `cc` failed: signal: 11 (SIGSEGV) (core dumped)
  ... "-fuse-ld=lld" ...
error: could not compile `getrandom` (build script)
```

This is a **QEMU emulation artifact**: `lld` segfaults compiling `getrandom`'s
build script under `qemu-x86_64` on an arm64 host. GitHub Actions' amd64 runner
is **native x86_64** — no QEMU, no lld segfault — so this failure is not
expected to reproduce in CI.

**Consequence for verification:** the full amd64 image cannot be built locally on
this hardware. The pg_cron fix is proven correct in isolation (see t1: `make`
produces `pg_cron.so` on amd64 with `-lintl` stripped), but the end-to-end
amd64 image build must be verified in **native CI**, not locally. t3's "build
locally" gate is therefore superseded by t5's CI gate for this environment.

**Do NOT** "fix" the lld segfault by changing the anvil stage — it is not broken
on the target (native amd64). Changing the linker to work around a QEMU bug
would be solving a problem CI does not have.

## t3 outcome (2026-07-10) — HALTED: local Docker environment failed

Local verification could not complete, for two stacked environment reasons —
neither a defect in the fix:

1. **QEMU/lld segfault** in the anvil stage (`cargo install cargo-pgrx`), as
   above — an arm64-emulating-amd64 artifact that native CI will not hit.
2. **Host disk exhausted** (`/System/Volumes/Data` at 100%, ~280Mi free of
   1.8Ti). The multi-hundred-MB QEMU build layers filled the volume; Colima's
   VM disk lives on it. `docker system prune` now fails with
   `input/output error` reading containerd's own blob store — the daemon's
   content store corrupted when the disk filled mid-write.

The scratch dir this session used only ~12K — the disk pressure is host-level
(user data + Colima VM image), NOT agent scratch. Not self-remediable without
deleting user data or restarting Colima, which is an operator decision.

**Operator action required before c001 can finish locally:**
- Reclaim host disk on `/System/Volumes/Data`.
- Likely `colima stop && colima start` (or delete/recreate the Colima VM) to
  repair the corrupted containerd blob store.
- Then re-run: `docker build --platform=linux/amd64 -f images/postgres18/Dockerfile .`

**The fix itself is complete and correct** (Dockerfile:100). The remaining
verification (t3 full build, t5 CI) is native-amd64 territory anyway, so the
authoritative gate is CI, not this arm64 host.

## Status
- t1 DONE — root cause = spurious `-lintl` in pg_cron Makefile; fix is removal.
- t2 DONE — `sed` applied to pgcron stage only (Dockerfile:100).
- t3 DONE — after disk recovery and Docker restart, the full local amd64 image
  built successfully; the earlier QEMU/lld failure did not recur.
- t4 DONE — full local arm64 image built successfully.
- t5 DONE — CI amd64 and arm64 jobs succeeded and the manifest job published.
