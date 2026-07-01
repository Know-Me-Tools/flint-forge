# Tasks — p3-c005-pg-graphql-pg18

## Change
OQ-3 resolution: verify/pin pg_graphql PG18 support

## Status: PENDING

---

## Task List

### T1 — Research pg_graphql PG18 release
- [ ] Check `https://github.com/supabase/pg_graphql/releases` for a PG18 tagged release
- [ ] If found: record version, confirm install method (PGXN or binary)
- [ ] If not found: identify most recent commit with PG18 CI passing; record SHA

### T2 — Write `docs/contracts/pg-graphql-version.md`
- [ ] Create the file with pinned version/SHA, install method, and any PG18 caveats
- [ ] Note verification date

### T3 — Verify/update `images/postgres18/Dockerfile`
- [ ] Read current pg_graphql install step in `images/postgres18/Dockerfile`
- [ ] If version differs from pinned: update to match `pg-graphql-version.md`
- [ ] Run `docker build` dry-run (or document that it will be run) to confirm

### T4 — Resolve OQ-3 in waypoint
- [ ] Update `.kbd-orchestrator/current-waypoint.json` `open_questions_remaining` — remove or mark OQ-3 as resolved
- [ ] Verify `cargo check --workspace` still passes (no source changes, but confirm no collateral breakage)

### T5 — Gate check
- [ ] Confirm `docs/contracts/pg-graphql-version.md` exists and is complete
- [ ] Mark this change `qa_passed` in `progress.json`
