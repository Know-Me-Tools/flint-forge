# Reflection — p16-v1.0-release-closure

**Phase status:** COMPLETE (6/6 changes) — with one disclosed partial gate
(c006) and one still-unresolved release decision that blocks shipping.

**Reflected:** 2026-07-16

---

## Headline

p16 replaced its own premise mid-flight, twice, and both times correctly.
`goals.md` was seeded assuming four changes (CI confirmation, tag+publish,
operator docs, KBD process hardening) built on the belief that v1.0.0 was
untagged and CI was green. Assessment (2026-07-09) found both false: v1.0.0
was already tagged and released on 2026-07-07, and the Postgres integration
CI job had **never** passed — 0/8 runs. The actual root cause (`pg_cron`
amd64 link failure) was a release-blocking defect in an *already-published*
release, not a pre-release gate. Spec replaced the four planned changes with
six defect-driven ones. All six shipped. The phase did what a phase should
do when its assumptions turn out wrong: it re-specced against reality instead
of executing a plan that no longer matched the codebase.

---

## Goal Achievement

| Goal (from `goals.md`, as re-specced) | Status | Evidence |
|---|---|---|
| p16-c001 — pg_cron amd64 link fix | **MET** | `b4264e9`; multi-arch manifest workflow green; both amd64 and arm64 images build |
| p16-c002 — Realtime fails closed, not silently empty | **MET** | `FabricChangeSource::watch()` returns `Err(Unavailable)`; default inverted to `listen`; live-Postgres integration test proves the default path delivers real events |
| p16-c003 — One green Postgres integration run | **MET** | 8 consecutive `success` conclusions found (not caused by this phase's work — c001 already fixed it; c003's job was verification, and it delivered honest evidence rather than re-running CI needlessly) |
| p16-c004 — Vulnerability disclosure channel | **MET** | GitHub private vulnerability reporting enabled and confirmed via `gh api`; `SECURITY.md`/`SUPPORT.md`/`CONTRIBUTING.md` authored with real, meetable commitments |
| p16-c005 — Reconcile documentation with code | **MET** | Stale `todo!()` claims corrected in three files; sweep found a fourth undocumented site |
| p16-c006 — Self-host operator guide (backup/restore/upgrade) | **PARTIAL — disclosed, not hidden** | Backup/restore genuinely executed against a live instance; upgrade path verified by commit inspection. The phase's own gate ("reach `/healthz` using only the docs") was **not** reached — blocked by a separate, pre-existing bug unrelated to restore. See "What Went Wrong (Instructively)" below. |

**5 of 6 goals fully MET. 1 of 6 (c006) MET on its authored deliverables but
NOT MET on its hardest acceptance criterion, and says so in its own
verification.md rather than claiming success.**

---

## What Actually Shipped

- `FabricChangeSource` fails closed instead of returning a silent empty
  stream (BREAKING, documented in `CHANGELOG.md`)
- Default realtime change source inverted to `listen` (working) from
  `fabric` (broken stub)
- `SECURITY.md`, `SUPPORT.md`, `CONTRIBUTING.md` — none existed before this
  phase
- GitHub private vulnerability reporting enabled on the repo (a real
  settings change, confirmed with the user before applying)
- `docs/operations/backup-restore.md` and `docs/operations/upgrading.md` —
  both new, both grounded in an actually-executed backup/restore cycle, not
  written from assumption
- Three stale documentation sites corrected (`rest/mod.rs:62`, README
  subscription claims ×2, `mounts_reflection_router.rs` test comments)
- Zero net-new code defects — everything landed clippy-pedantic clean,
  fmt-clean, with passing tests at every checkpoint

## What Was Found, Not Assumed

Two P0-severity bugs were discovered as a *side effect* of actually executing
c006's verification, not by static review:

1. **`vault.secrets`/`vault.access_log` rows are silently excluded from
   `pg_dump` entirely** — not just "unreadable without the DEK" as the
   original spec assumed, but absent from the dump altogether. Root cause:
   the tables are extension-owned (`pg_depend.deptype='e'`) and the
   extension never calls `pg_extension_config_dump()`. Confirmed with a real
   `pg_dump` → `pg_restore -l` showing zero TOC entries for the table's data,
   both with a full dump and an explicit `--data-only -t vault.secrets`.
   Filed as a follow-up (user has since started it in a separate session).
2. **`flint_meta.views()` does not exist**, so `fdb-gateway` panics on
   startup against any database — reproduced identically on a completely
   fresh, never-restored instance, proving it is unrelated to backup/restore
   and blocks every self-hosted operator from starting the gateway at all.
   Filed as a separate follow-up.

Neither was fixed in this phase — both are outside a docs-only change's
scope, and fixing pgrx extension code or the reflection engine mid-verification
would have silently expanded c006 past what was specced and asked. This is
the correct call: a docs change that starts patching Rust to make its own
gate pass would hide exactly the kind of undocumented risk this phase existed
to surface.

---

## Wait-Budget Discipline

**0 waits consumed this phase.** Budget was allocated at 3 (matching the
documented per-epoch limit; p15 overran at 6). c003 was flagged in the plan
as "the wait-budget risk — open-ended by design," expected to cost 1-3 waits.
It cost zero: the job was already green by the time c003 was picked up (8
consecutive successes, 2026-07-13 through 2026-07-14), a fact `gh run list`
surfaced in under a minute. This is a genuine efficiency win, but it is also
a **process signal worth naming**: the job turned green roughly three days
before this phase started tracking it, and neither `progress.json` nor the
waypoint reflected that until this session checked. That is the same
position-drift class p16's own inherited-debt item 5 exists to fix — it
recurred, in miniature, within the phase meant to close it out.

---

## Inherited Debt — Resolution Status

| # | Debt (from p15) | Status after p16 |
|---|---|---|
| 1 | No artifact-refiner QA logs exist | **STILL OPEN.** No `.refiner/artifacts/` entries were produced for any p16 change. p16's `goals.md` proposed either enforcing this or amending the KBD contract to stop requiring it — neither happened; the six re-specced changes did not include the original c004 (KBD process hardening) that would have addressed this. |
| 2 | p15 wait budget overran (6 vs. 3) | **RESOLVED for p16** (0/3 spent), but the underlying discipline gap (goal #4 in original goals.md — enforce the budget) was never implemented as tooling; it held this phase by manual attention only. |
| 3 | k6 baselines are local, not staging | **STILL OPEN**, explicitly deferred in `goals.md` ("Deferred — requires production-like infrastructure not currently available"). Correctly scoped out, not silently dropped. |
| 4 | Native KBD changes tracked but never archived | **RESOLVED for p16.** All 6 changes moved to `.kbd-orchestrator/changes/archive/<date>-<id>/` this phase, via the same `kbd-apply verify`/`archive` driver calls throughout. |
| 5 | Position files drifted 11-12 phases stale | **RESOLVED for p16** — waypoint and progress.json were kept current turn-by-turn. Not verified whether `position.json`/`position-reminder.txt` (the files debt item 5 specifically names) were kept in sync, since this session interacted primarily with `current-waypoint.json`. **Recommend an explicit check before closing this line item.** |

Net: 2 of 5 debt items resolved this phase (archiving, position drift for
the files touched), 1 improved but not tooled (wait budget), 2 still open
(artifact-refiner QA, staging k6). The KBD process-hardening change that
would have addressed items 1 and 5 as tooling (not just this phase's
discipline) was dropped from the re-specced plan — worth deciding explicitly
whether to schedule it, rather than letting it silently re-drop.

---

## What Went Wrong (Instructively)

c006's spec set a real bar: "do not ship a theorized restore." Meeting that
bar honestly required admitting the environment didn't cooperate twice —
Docker Desktop's Kubernetes wasn't running (switched to plain Docker, with
user confirmation) and the gateway itself couldn't start against a fresh
database for reasons that had nothing to do with backup/restore (the
`flint_meta.views()` bug). Both times, the choice was: paper over it and
claim success, or stop, ask, and document exactly what did and didn't work.
The second path was taken both times. The resulting docs are more valuable
for it — an operator following `backup-restore.md` today will hit the exact
`--clean --if-exists` requirement and the vault-table gap the doc now warns
about, because those are the actual failures encountered while writing it,
not hypothesized ones.

---

## Artifact Quality Summary

No artifact-refiner logs exist for this phase (see Inherited Debt #1). This
table cannot be populated from `.refiner/` data; it is omitted rather than
fabricated. Quality evidence available instead: `cargo clippy --workspace -D
warnings` and `cargo fmt --check` were run and passed at every code-touching
checkpoint (c001, c002, c005); `cargo test` suites relevant to touched crates
passed (25/25 fdb-realtime, 1/1 mounts_reflection_router, 2/2 live-Postgres
integration tests for c002 and c006's exercise).

---

## Lessons Captured

1. **Re-specifying mid-phase against discovered reality beats executing a
   stale plan.** Both the initial pivot (goals.md's 4 changes → spec's 6) and
   c006's honest partial-completion are instances of the same discipline:
   trust what the codebase/environment actually shows over what a prior
   document assumed.
2. **"Verify, don't re-execute" saved a wait.** c003 could have consumed 1-3
   waits re-running CI from scratch. Checking `gh run list` first found the
   job was already green, for reasons unrelated to this phase's own work,
   and cost zero waits. Always check current state before spending budget to
   reproduce it.
3. **Executing an operational procedure surfaces bugs static review
   won't.** Both P0 findings (`pg_dump` vault exclusion, `flint_meta.views()`
   missing) were found only because the restore was actually run, not
   theorized. This validates the spec's "do not ship a theorized restore"
   requirement as more than process theater.
4. **A partial gate, disclosed, is more valuable than a false full pass.**
   c006's verification.md explicitly marks the `/healthz` criterion
   unreached and explains why. This is the correct failure mode for a
   phase whose entire purpose is closing the gap between what's claimed and
   what's true (see p15's false "green in CI" claim that started this whole
   phase).
5. **Enabling a shared GitHub repo setting (private vulnerability
   reporting) is a real, team-visible action** — correctly gated behind an
   explicit confirmation rather than assumed-authorized by the parent task.

---

## Recommended Next Phase

**P0 — Release decision.** The `v1.0.0` re-tag vs. `v1.0.1` question has now
been open across five consecutive stages (assess, analyze, spec, plan,
reflect) without resolution. It is the single remaining blocker to any actual
release action: the current `v1.0.0` tag points at a commit predating the
amd64 fix, and the GitHub Release has zero published assets, meaning nothing
external is currently pinned to the broken state — a re-tag is technically
low-risk, but this is a judgment call for the user, not KBD. **Surface this
explicitly before starting further phase work; do not let it drift to a
sixth stage.**

**P0 — Two filed follow-ups.** `vault.secrets` pg_dump exclusion (already
started by the user in a separate session) and `flint_meta.views()` missing
(gateway startup blocker for every self-hosted operator — not yet started).
The second is more severe: it means the operator guide's own `/healthz`
gate cannot pass for anyone until it's fixed. Recommend prioritizing it
immediately after the release decision, since it blocks the exact "clean
install works" claim this phase was chartered to make true.

**P1 — Revisit KBD process hardening.** The original c004 (artifact-refiner
enforcement + position-staleness guard) was dropped when the phase
re-specced around defects instead. It addressed real, still-open debt (items
1 and 5). Either schedule it explicitly or make an informed decision to drop
the artifact-refiner requirement from the KBD contract — the current state
(silently not doing it, phase after phase) is worse than either choice made
deliberately.

**P1 — Staging k6 validation.** Still deferred, still blocked on
infrastructure that doesn't exist. No change in status; re-confirm it's
still the right call to defer before any GA claim beyond "self-hosted beta."
