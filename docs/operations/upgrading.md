# Upgrading Flint Forge

`CHANGELOG.md` states this project follows [Semantic Versioning
2.0.0](https://semver.org/spec/v2.0.0.html). This document is where that
claim becomes a concrete procedure.

## Supported Upgrade Paths

**Honest current state: only sequential upgrade (N → N+1 tagged release) is
verified.** Skip-version upgrades (e.g. jumping two major versions at once)
are not tested and not recommended until this document says otherwise.

| From | To | Status |
|------|-----|--------|
| v0.10.0 | v1.0.0 | Verified by inspection (see below) — no schema migration required, one breaking Rust-level fix |
| Any → any (general policy) | | Upgrade one tagged release at a time; do not skip versions |

## What "Upgrade" Means Here

Flint Forge ships as container images (`fdb-gateway`, `fke-server`, the
Postgres image with pgrx extensions baked in) plus a set of `sqlx` migrations
applied at gateway startup. An upgrade has up to three independent parts:

1. **Application binaries** — new `fdb-gateway`/`fke-server` images.
2. **Database schema** — `sqlx migrate run`, applied automatically by
   `fdb-gateway` on startup (see `crates/fdb-gateway/src/main.rs`).
3. **pgrx extensions** — `ALTER EXTENSION <name> UPDATE`, run manually.

## v0.10.0 → v1.0.0, Concretely

Verified by inspecting the actual commit range (`git log v0.10.0..v1.0.0`,
15 commits) as part of authoring this document:

- **No new SQL migrations were added between these two tags.** The schema is
  unchanged; there is nothing for `sqlx migrate run` to apply that a
  v0.10.0 database doesn't already have caught up to.
- **One breaking change, pure Rust, no migration needed:**
  `PgBackend::acquire` (`crates/fdb-postgres`) had a critical bug where RLS
  context propagation (`SET LOCAL ROLE`, the 6 GUCs) silently failed at
  runtime because Postgres `SET` statements don't accept bind parameters.
  Fixed in commit `35fdf01` by switching to `set_config()` calls. **This is
  a bug fix, not a new feature** — RLS enforcement was broken in v0.10.0 and
  is correct in v1.0.0. There is no operator action required beyond
  deploying the new binary; the fix takes effect on restart.

**Procedure:**

```bash
# 1. Deploy the new fdb-gateway / fke-server images (Helm: bump image.tag).
# 2. Restart. fdb-gateway runs `sqlx migrate run` automatically on startup —
#    it is a no-op here since there are no new migrations for this range.
# 3. No pgrx extension version changes for this range (all extensions remain
#    at 0.1.0 — see "Extension Versioning" below).
```

No manual schema steps. No `ALTER EXTENSION` calls needed for this specific
range.

## General Upgrade Procedure (schema present)

For a future release that *does* add migrations:

```bash
# Back up first — see docs/operations/backup-restore.md.
pg_dump -h <host> -U <user> -d flint -Fc -f pre-upgrade.dump

# Deploy new binaries; fdb-gateway applies pending migrations on startup.
# Confirm what will run before deploying, if you want to review it first:
sqlx migrate info --source migrations --database-url "$DATABASE_URL"

# After startup, confirm:
curl -f http://localhost:8080/healthz
```

`scripts/verify-migrations.sh` (run in CI) guarantees migration filenames
have a strict, gap-free numeric prefix sequence with no duplicates — this is
what prevents the collision class that broke an earlier boot (two migrations
sharing prefix `0005`, since fixed).

## Extension Upgrade (`ALTER EXTENSION ... UPDATE`)

**Honest current state: this has never been exercised.** All five pgrx
extensions are at `default_version = '0.1.0'`
(`ext-flint-auth`, `ext-flint-hooks`, `ext-flint-meta`, `flint_llm`,
`flint_vault`) — there is no prior extension version bump in this project's
history to base a tested procedure on. When an extension does ship a new
version, the general Postgres pattern applies:

```sql
ALTER EXTENSION ext-flint-auth UPDATE TO '<new-version>';
ALTER EXTENSION ext-flint-hooks UPDATE TO '<new-version>';
ALTER EXTENSION ext-flint-meta UPDATE TO '<new-version>';
ALTER EXTENSION flint_llm UPDATE TO '<new-version>';
ALTER EXTENSION flint_vault UPDATE TO '<new-version>';
```

This requires the new extension `.so` and SQL upgrade script
(`<name>--0.1.0--<new-version>.sql`) to already be installed on the Postgres
host/image — i.e., you must be running the new Postgres image *before*
running `ALTER EXTENSION`. Until this project publishes its first extension
version bump with a real upgrade script, treat this section as the general
pattern, not a tested Flint Forge procedure.

## Rollback

**Rollback is not supported.** There is no down-migration tooling in this
project (`sqlx` migrations here are forward-only — check `migrations/` for
any `.down.sql` files; none exist as of v1.0.0). If an upgrade needs to be
reverted:

1. Restore from the pre-upgrade backup (see
   `docs/operations/backup-restore.md`) — this is the only supported
   rollback path.
2. Redeploy the prior version's images against the restored database.

Plan backups as part of every upgrade, not as an afterthought — see the
Backup and Restore doc's RTO/RPO section for what that means for your
recovery time.

## Breaking-Change Policy

Tied to the SemVer claim in `CHANGELOG.md` and the supported-versions table
in `SECURITY.md`:

- Breaking changes are called out under a `BREAKING` heading in
  `CHANGELOG.md` (see the `[Unreleased]` section for the current example:
  p16-c002's realtime change-source default inversion).
- A breaking change bumps the major version, per SemVer.
- Security fixes are backported to the latest minor of the **current** major
  version only (see `SECURITY.md` — there is no long-term-support branch).
  This means an operator wanting security fixes without breaking changes
  should stay on the latest minor of their current major, not skip to a new
  major purely for a fix.
