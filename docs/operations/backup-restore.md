# Backup and Restore

Flint Forge's Postgres instance is not a stock database. It carries pgrx
extensions with their own state, an encrypted secret store whose key lives
**outside** the database, and background job state. A naive `pg_dump | psql`
is **not** a correct backup procedure. This document describes what a correct
one looks like, and states plainly where it currently falls short.

This procedure was executed against a real instance on 2026-07-16 (plain
Docker, `flint-forge-pg:18`, Postgres 18.4) — not theorized. See "What Was
Actually Verified" at the end for the exact evidence.

## What `pg_dump` Captures — and What It Does Not

A standard `pg_dump -Fc` of the `flint` database captures:

- All ordinary application schemas/tables/data (e.g. anything under your own
  schemas, `flint_a2ui`, `flint`, `flint_meta` metadata tables, `cron.job` /
  `cron.job_run_details` state)
- The `CREATE EXTENSION` statements for all installed extensions
- Encrypted ciphertext, for tables that are dumped

It does **not** capture:

1. **The `flint_vault` DEK.** The Data Encryption Key never lives in Postgres
   — it is generated once, KMS-wrapped, stored as a Kubernetes secret
   (`FLINT_VAULT_DEK_WRAPPED`), and unwrapped into pod memory at startup (see
   `docs/contracts/vault-kms.md`). `pg_dump` has no access to it and cannot
   capture it. **A database restored without separately restoring the wrapped
   DEK (and the KMS key that unwraps it) will have zero readable secrets.**
   The rows may exist, but `vault.get_secret()` fails closed with "no key
   configured."

2. **⚠️ `vault.secrets` and `vault.access_log` row data — not just the DEK,
   the rows themselves.** This was discovered during verification of this
   document, not assumed in advance: `vault.secrets` is a table owned by the
   `flint_vault` extension (`pg_depend.deptype = 'e'`). Postgres excludes
   extension-owned table **data** from `pg_dump` by default — this is
   standard Postgres behavior for extension "config tables," but
   `flint_vault` does not currently call
   `pg_catalog.pg_extension_config_dump()` to opt back in. The result: a
   standard `pg_dump -Fc` of the whole database contains **zero rows** of
   `vault.secrets`, encrypted or not. This is tracked as a bug (not a
   documented limitation to live with) — see "Known Gap" below. Until it is
   fixed, use the explicit workaround in step 2 of the backup procedure.

## Backup Procedure

### 1. Database dump

```bash
pg_dump -h <host> -p <port> -U <user> -d flint -Fc -f flint.dump
```

Use `-Fc` (custom format) — it supports selective and parallel restore, which
matters for a database this size with multiple extensions.

### 2. Vault tables (required — see "Known Gap" above)

Because `vault.secrets` and `vault.access_log` are excluded from step 1, back
them up explicitly with `\copy`, which is not subject to the extconfig
restriction:

```bash
psql -h <host> -p <port> -U <user> -d flint \
  -c "\copy vault.secrets TO 'vault-secrets.csv' WITH (FORMAT csv, HEADER true)"
psql -h <host> -p <port> -U <user> -d flint \
  -c "\copy vault.access_log TO 'vault-access-log.csv' WITH (FORMAT csv, HEADER true)"
```

Both files contain **ciphertext only** — `secret` is XChaCha20-Poly1305
ciphertext, not plaintext. They are still sensitive (the row structure and
metadata are not secret, but treat them at the same handling tier as the
main dump).

### 3. The vault DEK

This is **not a database operation.** Preserve, independently of the
database backup:

- The KMS-wrapped DEK (the value in the `FLINT_VAULT_DEK_WRAPPED` Kubernetes
  secret, or your `FLINT_VAULT_ROOT_KEY` if running in dev mode — see
  `docs/contracts/vault-kms.md` §5). Back this up as you would any other
  credential: your existing secrets-management process, not a database
  backup.
- Access to the KMS key that unwraps it (Azure Key Vault `flint-vault-kek` in
  production). This is normally already covered by your KMS provider's own
  durability guarantees — confirm it is.

**If you lose the wrapped DEK and the KMS key that unwraps it, the encrypted
secrets in `vault.secrets` are permanently unrecoverable, even with a perfect
database backup.** This is by design — it is what makes it a real KMS-backed
secret store rather than an obfuscation layer — but it means your backup
procedure is incomplete if it only covers the database.

### 4. Extension and job state

No separate action needed — `cron.job` (scheduled job definitions) and
`cron.job_run_details` (job run history) are ordinary tables captured by step
1. `flint_meta` cache tables are rebuilt from DDL triggers and do not need
special handling; they will repopulate from the restored schema on next
reflection cycle.

## Restore Procedure

### 1. Provision a target instance

Start a fresh `flint-forge-pg:18` (or Helm-deployed) instance. **Do not**
assume it is empty — the image provisions its own schemas and pgrx extension
config tables (including baseline `cron.job` entries) at container init. A
plain restore into this "fresh" instance will collide on `CREATE SCHEMA` and
duplicate-key errors.

### 2. Restore the database dump with `--clean`

```bash
pg_restore -h <host> -p <port> -U <user> -d flint \
  --no-owner --clean --if-exists flint.dump
```

`--clean --if-exists` drops the pre-provisioned objects before recreating
them from the dump, avoiding the collision from step 1. This was necessary
in verification — a restore without these flags failed with 7 errors
(`schema "X" already exists`, duplicate `cron.job` keys).

### 3. Restore the vault tables

```bash
psql -h <host> -p <port> -U <user> -d flint \
  -c "\copy vault.secrets FROM 'vault-secrets.csv' WITH (FORMAT csv, HEADER true)"
psql -h <host> -p <port> -U <user> -d flint \
  -c "\copy vault.access_log FROM 'vault-access-log.csv' WITH (FORMAT csv, HEADER true)"
```

### 4. Restore the vault DEK

Configure the target instance's environment with the **same** wrapped DEK
(`FLINT_VAULT_DEK_WRAPPED`) and unwrap command/KMS access as the source. In
dev mode, the same `FLINT_VAULT_ROOT_KEY`.

### 5. Verify

```bash
psql -h <host> -p <port> -U <user> -d flint -c "SELECT vault.get_secret('<a known secret name>');"
```

If this returns the correct plaintext, the DEK restore succeeded. If it
errors with "no key configured," the DEK/unwrap path is wrong. If it errors
with "no secret named," the row restore (step 3) did not happen.

## RTO / RPO Guidance

This procedure has no automation — it is manual, and its duration scales with
database size. Rough guidance for a self-hosted operator sizing their own
backup cadence:

- **RPO (data loss window):** equal to your backup frequency. There is no
  continuous/WAL-based backup documented here (out of scope — see
  Non-Goals). If you need point-in-time recovery, layer `pg_basebackup` +
  WAL archiving on top of this procedure; that is standard Postgres PITR and
  is not Flint-specific.
- **RTO (recovery time):** dominated by `pg_dump`/`pg_restore` time for your
  data volume, plus manual DEK/KMS reconfiguration (minutes, not automated).
  Test your own RTO against your own data size — this procedure was verified
  functionally correct, not benchmarked for scale.

## Known Gap (tracked separately, not blocking this document)

`vault.secrets` and `vault.access_log` should call
`pg_catalog.pg_extension_config_dump()` during extension install so a
standard `pg_dump` captures them without the manual `\copy` workaround in
step 2. This is a real defect in `ext-flint-vault`, filed as a follow-up. The
workaround in this document is required until it lands — do not skip step 2.

## What Was Actually Verified

Executed 2026-07-16 against `flint-forge-pg:18` (Postgres 18.4) via plain
Docker (not Helm/Kubernetes — Docker Desktop's k8s API was unavailable in
this environment; the procedure is identical at the Postgres level regardless
of orchestrator):

1. Fresh instance, all 11 `sqlx` migrations applied cleanly.
2. `flint_vault` extension enabled; created a secret via
   `vault.create_secret()` with `FLINT_VAULT_ROOT_KEY` set (dev-mode DEK).
   Confirmed `vault.get_secret()` decrypts it correctly.
3. Added ordinary application data (`demo.widgets`, 3 rows) and a `pg_cron`
   job, alongside the pre-existing production jobs
   (`webhook-outbox-gc`, `meta-full-refresh`, `webhook-outbox-processor`).
4. Ran `pg_dump -Fc`. Confirmed via `pg_restore -l` that `vault.secrets` data
   was **absent** from the dump (the Known Gap above) — this is how the gap
   was discovered, not assumed.
5. Confirmed the plaintext secret does not appear anywhere in the dump file
   (`strings flint.dump | grep <plaintext>` — zero matches, as expected;
   what's dumped for other extension-owned tables is ciphertext, never
   plaintext).
6. Took the `\copy` workaround backup of `vault.secrets` /
   `vault.access_log`.
7. Restored into a fresh target instance: `pg_restore --clean --if-exists`
   succeeded without the workaround flags on the first attempt failing with 7
   errors as described above; second attempt with the flags succeeded
   cleanly.
8. Restored the vault CSVs via `\copy ... FROM`.
9. **Without** the DEK set on the target: `vault.get_secret()` correctly
   failed closed with "no key configured" — proving the documented failure
   mode is real, not theoretical.
10. **With** the correct DEK set on the target: `vault.get_secret()` returned
    the correct plaintext. `demo.widgets` had all 3 rows. `cron.job` had all
    4 jobs (3 pre-existing + the test job).

**Not verified in this pass:** step "reach `/healthz` via the gateway" from
the phase's acceptance criteria. `fdb-gateway` failed to start against
*both* the restored instance and a freshly-migrated, never-restored
instance, with an unrelated pre-existing error
(`flint_meta.views()` does not exist). This is **not a restore defect** — it
reproduces identically on a database that was never touched by this
procedure — and is filed as a separate, higher-severity bug (it blocks any
operator from starting the gateway at all, not just a restored one). The
backup/restore procedure above is verified correct at the Postgres layer;
gateway startup is a separate, currently-broken concern.
