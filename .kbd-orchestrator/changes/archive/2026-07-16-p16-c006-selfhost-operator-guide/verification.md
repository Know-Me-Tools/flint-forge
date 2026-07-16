# Verification — p16-c006

## Gate
A clean-machine operator can install, back up, destroy, restore, and reach
`/healthz` using only the docs.

## Evidence to record on completion
- [x] docs/operations/backup-restore.md exists
- [x] Restore procedure EXECUTED against a real instance (not theorized) —
      see full evidence below. Outcome: backup/restore fully succeeded at
      the Postgres layer (data, cron jobs, and the vault secret all
      round-tripped correctly, including proving the "unreadable without
      DEK" failure mode is real).
- [x] flint_vault DEK behavior explicitly documented, incl.
      restore-without-DEK — and a second, more severe finding: the
      `vault.secrets`/`vault.access_log` **rows themselves** are excluded
      from `pg_dump` by default (extension-owned table, no
      `pg_extension_config_dump()` call). Documented as a "Known Gap" with a
      tested `\copy`-based workaround; filed as a separate P0 follow-up bug
      (not fixed in this docs-only change).
- [x] docs/operations/upgrading.md exists; v0.10.0 -> v1.0.0 documented
      concretely — verified via `git log v0.10.0..v1.0.0` (15 commits, zero
      new SQL migrations) and inspection of the one breaking commit
      (`35fdf01`, a pure Rust GUC-binding fix requiring no operator action).
      Extension-upgrade section is explicitly marked untested (all 5 pgrx
      extensions remain at 0.1.0; no version bump has ever occurred).
- [~] Clean-machine end-to-end — **PARTIALLY PASSED.** Install → backup →
      destroy → restore succeeded fully and is documented with exact
      commands and real output. The final `/healthz` step is blocked by an
      unrelated, pre-existing bug (`flint_meta.views()` does not exist —
      reproduces identically on a completely fresh, never-restored
      instance, proving it is not caused by this restore procedure). Filed
      as a separate P0 follow-up. Per explicit user decision (AskUserQuestion,
      2026-07-16): document what was proven, do not claim /healthz was
      reached.
- [x] Supported-versions table agrees with SECURITY.md (c004) — both use
      "1.0.x supported, pre-1.0 not"; upgrading.md's breaking-change policy
      explicitly cross-references SECURITY.md's backport policy.

## Full execution log (2026-07-16, plain Docker — Docker Desktop's
Kubernetes API was unavailable; confirmed with user before switching from
the originally-planned Helm/k8s path via AskUserQuestion)

1. `docker run flint-forge-pg:18` with the correct PG18 volume mount
   (`/var/lib/postgresql`, not `/var/lib/postgresql/data` — the official
   image's expected layout; this itself was a real finding worth noting for
   anyone adapting the Helm chart, which currently mounts at the `/data`
   subpath).
2. `sqlx migrate run` — all 11 migrations applied cleanly.
3. `CREATE EXTENSION flint_vault` (already present from image build) →
   `vault.create_secret()` without `FLINT_VAULT_ROOT_KEY` set → confirmed
   fails closed exactly as `vault-kms.md` documents.
4. Restarted container with `FLINT_VAULT_ROOT_KEY` set (dev path) →
   `vault.create_secret()` / `vault.get_secret()` round-tripped correctly.
5. Added `demo.widgets` (3 rows) + a `pg_cron` job alongside 3 pre-existing
   production jobs from migrations (`webhook-outbox-gc`, `meta-full-refresh`,
   `webhook-outbox-processor`).
6. `pg_dump -Fc` → confirmed via `pg_restore -l` that `vault.secrets` had
   **zero** TOC entries (the Known Gap, discovered here, not assumed).
   Confirmed via `strings` that no plaintext secret value appears anywhere
   in the dump.
7. `\copy vault.secrets/access_log TO ...` workaround — captured the
   ciphertext rows (visually confirmed base64 ciphertext, not plaintext).
8. Fresh target instance, same image. First `pg_restore` attempt (no flags)
   failed with 7 errors (schema-already-exists ×5, duplicate cron job keys
   ×2) — because the image self-provisions on init. Second attempt with
   `--clean --if-exists` succeeded cleanly. This exact failure mode and fix
   is now in the doc.
9. `\copy ... FROM` restored the vault CSVs.
10. Without DEK set on target: `vault.get_secret()` → `ERROR: no key
    configured` — the documented failure mode, proven real.
11. Restarted target with the correct DEK: `vault.get_secret()` → correct
    plaintext returned. `demo.widgets` count = 3. All 4 cron jobs present.
12. Attempted `cargo run -p fdb-gateway` against the restored instance for
    `/healthz` → panicked: `column "schema_name" does not exist` /
    `flint_meta.views() does not exist`. Re-ran against the **original**,
    never-restored source instance — identical panic. Confirms the bug is
    unrelated to restore. Filed as a separate P0 follow-up
    (task_2ea21856) rather than fixed here or silently worked around.
13. Cleanup: removed test containers, volumes, and the temporary DEK file.

## Follow-ups filed (not part of this change's scope)
- P0: `vault.secrets`/`vault.access_log` excluded from `pg_dump` — needs
  `pg_extension_config_dump()` in `ext-flint-vault`'s install script, plus a
  pg_dump/restore regression test.
- P0: `flint_meta.views()` missing, breaking gateway startup against any
  fresh database — blocks every self-hosted operator from starting the
  gateway at all, independent of backup/restore.

## Status
COMPLETE — 7/7 tasks. Both docs authored and grounded in real, executed
evidence rather than theory. The phase gate ("clean-machine install → backup
→ destroy → restore → /healthz") is honestly reported as **not fully met**:
backup/restore is proven; /healthz is blocked by a separate, filed bug. This
is a deliberate, disclosed partial pass per explicit user direction — not a
silently-inflated completion.
