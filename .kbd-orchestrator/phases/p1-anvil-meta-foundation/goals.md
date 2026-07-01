# Goals — p1-anvil-meta-foundation

**Phase 1: Flint Anvil + Meta Foundation**

## Primary goals

- Implement `ext-flint-auth` pgrx extension: `auth.*` SQL helpers + GUC contract (pgrx 0.12/pg17)
- Implement `ext-flint-hooks` standard tier: webhook registry + dispatch trigger + pg_net + Option-3 HMAC
- Implement `ext-flint-hooks` durable tier: outbox table + dispatcher BGW + SKIP LOCKED retry
- Pin JWT contract: `docs/contracts/jwt-contract.md` — exact claim shape from flint-gate + service-identity format
- Add `pg_cron` to `images/postgres18/Dockerfile`
- Implement `ext-flint-vault` KMS unwrap: Azure Key Vault managed identity v1

## New goals (from revised plan RFC-FORGE-PHASES-002)

- Implement `ext-flint-meta` pgrx extension (pgrx 0.18.1/pg18):
  - Cache tables: `cache_tables`, `cache_columns`, `cache_relationships`, `cache_functions`, `cache_policies`, `cache_types`
  - Version tracking: `flint_meta.schema_version`
  - Keto tuple storage: `flint_meta.keto_tuples` + indexes
  - Vault key metadata: `flint_meta.vault_keys`, `vault_key_assignments`
- Implement DDL event triggers: `ddl_command_end` → `refresh_cache()` → version++ → `pg_notify('meta_runtime', ...)`
- Implement SQL-callable reflection functions: `flint_meta.tables()`, `columns()`, `relationships()`, `functions()`, `version()`, `check_permission()`, `set_identity()`
- Implement AG-UI descriptor functions: `flint_meta.agui_descriptor()`, `flint_meta.openapi()`
- Gate test: `sqlx::PgListener` on `meta_runtime` receives notification after `CREATE TABLE`; version increments

## Gate

`flint_auth` passes RLS end-to-end; `flint_hooks` fires a signed webhook through flint-gate;
`flint_meta` extension installs, cache tables populate, event trigger fires on `CREATE TABLE`
and increments version counter, NOTIFY reaches a test LISTEN client within 5s.

## Reference

- Revised plan: `docs/FLINT-PHASE-PLAN-REVISED.md` §Phase 1
- RFC-FORGE-META-001: `docs/FLINT-META-EXTENSION-PLAN.md` §4 (schema), §4.3 (triggers), §4.4 (JWT propagation)
- pgrx 0.18.1 single-compile migration: remove `src/bin/pgrx_embed.rs`, switch to `crate-type = ["cdylib"]`
- Known DDL gap: `CREATE TABLE AS` does not fire `ddl_command_end` in PG ≤ 15; document in `docs/contracts/meta-trigger-coverage.md`
- LISTEN/NOTIFY: must implement reconnect loop — `sqlx::PgListener` does not auto-resubscribe on connection loss
