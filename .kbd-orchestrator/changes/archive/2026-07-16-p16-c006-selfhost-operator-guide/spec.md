# p16-c006 — Self-host operator guide: backup, restore, upgrade

**Phase:** p16-v1.0-release-closure
**Priority:** P1 — Tier 1, beta-blocker
**Scope:** `docs/` (new operator documentation)
**Depends on:** p16-c001 (a pullable image is a precondition for any install doc)
**Delivery model:** self-hosted OSS

---

## Problem

Two gaps, both fatal to a self-hosted beta:

### 1. No backup / restore / PITR procedure

Nothing in `docs/` describes how an operator backs up or restores a Flint Forge
Postgres instance. This is the **largest undocumented risk** found in the whole
analysis. Anyone trusting a database with data will ask, and there is no answer
on disk.

This is materially harder than stock Postgres, because the instance carries:
- five pgrx extensions with their own catalog state
- `flint_vault` — encrypted secrets whose **KMS-wrapped DEK is not in the
  database dump**. A `pg_dump` restore without the DEK yields unreadable
  secrets. This must be stated explicitly.
- `flint_meta` cache tables rebuilt from DDL triggers
- `pg_cron` job state

A naive `pg_dump | psql` is **not** a correct restore. The doc must say so.

### 2. No upgrade path

`CHANGELOG.md` claims adherence to Semantic Versioning 2.0.0. No migration guide
exists between any two versions. `sqlx` migration ordering was only repaired in
p15-c002 (`scripts/verify-migrations.sh`, 11 migrations). An operator on v0.10.0
has no documented route to v1.0.0.

## Change

Author `docs/operations/backup-restore.md` and `docs/operations/upgrading.md`.

**Backup/restore must cover:** what `pg_dump` does and does not capture; the
`flint_vault` DEK — where it lives, how to back it up, what happens without it;
extension/catalog state; a **tested** restore procedure (run it, don't theorize
it); RTO/RPO guidance an operator can reason about.

**Upgrading must cover:** supported upgrade paths (N-1? any-to-any?); the
`sqlx migrate run` sequence and its ordering guarantee; extension upgrade
(`ALTER EXTENSION … UPDATE`) for all five pgrx extensions; rollback, or an
explicit statement that rollback is unsupported; breaking-change policy tied to
the SemVer claim.

## Acceptance Criteria

1. `docs/operations/backup-restore.md` exists and its restore procedure has been
   **executed at least once against a real instance**, with the result recorded.
2. It explicitly addresses the `flint_vault` DEK and states the consequence of
   restoring without it.
3. `docs/operations/upgrading.md` exists and documents v0.10.0 → v1.0.0
   concretely, not abstractly.
4. A clean-machine operator can install, take a backup, destroy the instance,
   restore it, and reach `/healthz` — using only the docs.
5. The supported-versions table in `SECURITY.md` (c004) and the upgrade policy
   here do not contradict each other.

## Non-Goals

- Automated backup tooling. Documentation first; automation is a later phase.
- HA / replication / failover.
- Managed-service concerns (status page, on-call). Out of scope per the
  self-hosted OSS delivery model.

## Verification Command

Manual, and it must actually be performed:

```bash
# on a clean machine, following ONLY the docs:
helm install flint-forge deploy/helm/flint-forge
# ... take backup per docs/operations/backup-restore.md
helm uninstall flint-forge && kubectl delete pvc --all
# ... restore per the doc
curl -f http://localhost:8080/healthz
```

## Risk

**High effort, high value.** Days, not hours. The `flint_vault` DEK interaction
is the part most likely to be discovered wrong *during* the first real restore —
which is exactly why criterion 1 demands the procedure be executed, not written.

## Open Questions

- **Does a backup/restore procedure exist outside this repository?** (from
  `analysis.md`) If ops runbooks exist elsewhere, this collapses to a doc link
  plus verification. If not, it is genuine engineering work. **Answer before
  estimating.**
- Is v0.10.0 → v1.0.0 actually supported? p35-c004 landed a `BREAKING` change to
  `PgBackend::acquire` RLS setup. The upgrade may require manual steps.
