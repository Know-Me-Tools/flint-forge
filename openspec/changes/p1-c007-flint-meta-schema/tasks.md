# p1-c007 — Tasks

## Crate scaffold
- [ ] Create `crates/ext-flint-meta/Cargo.toml` — copy ext-flint-vault Cargo.toml structure; change name to `ext-flint-meta`, remove crypto dependencies, keep pgrx = "=0.18.1" pg18
- [ ] Create `crates/ext-flint-meta/src/lib.rs` — `pgrx::pg_module_magic!()`, `extension_sql_file!("sql/flint_meta.sql")`, `#[pg_extern] fn flint_meta_version() -> &'static str { "0.1.0" }`
- [ ] Verify no `src/bin/pgrx_embed.rs` created — pgrx 0.18.1 single-compile uses `cdylib` only
- [ ] Create `crates/ext-flint-meta/src/schema.rs` — cache table SQL constants (as `extension_sql!` or SQL file)
- [ ] Create `crates/ext-flint-meta/src/version.rs` — schema_version table SQL + `increment_version()` function
- [ ] Create `crates/ext-flint-meta/src/keto.rs` — keto_tuples table + indexes SQL
- [ ] Create `crates/ext-flint-meta/src/vault_meta.rs` — vault_keys + vault_key_assignments SQL

## SQL file
- [ ] Create `crates/ext-flint-meta/sql/flint_meta.sql` with all CREATE TABLE statements in dependency order:
  - [ ] `CREATE SCHEMA IF NOT EXISTS flint_meta`
  - [ ] `cache_tables`, `cache_columns`, `cache_relationships`, `cache_functions`, `cache_policies`, `cache_types`
  - [ ] `schema_version` with seed `INSERT (version=1)`
  - [ ] `keto_tuples` + 2 indexes
  - [ ] `vault_keys`, `vault_key_assignments`

## Schema security
- [ ] `REVOKE ALL ON ALL TABLES IN SCHEMA flint_meta FROM PUBLIC`
- [ ] `GRANT SELECT ON flint_meta.cache_* TO authenticated, anon`
- [ ] `GRANT ALL ON flint_meta.keto_tuples TO service_role`
- [ ] `GRANT ALL ON flint_meta.vault_keys, flint_meta.vault_key_assignments TO vault_admin`

## Tests
- [ ] Write pgrx `#[pg_test]` for version: `SELECT flint_meta.version()` returns `1`
- [ ] Write pgrx `#[pg_test]` for cache_tables INSERT/SELECT roundtrip
- [ ] Write pgrx `#[pg_test]` for keto_tuples INSERT/SELECT/DELETE roundtrip

## Verification
- [ ] Add to root `Cargo.toml` exclude list if not already excluded: `"crates/ext-flint-meta"`
- [ ] Run `cargo pgrx run -p ext-flint-meta --features pg18` — starts PG18 with extension installed
- [ ] `\dt flint_meta.*` shows all expected tables
- [ ] `SELECT flint_meta.flint_meta_version()` returns `'0.1.0'`
- [ ] GATE: all tables exist; version() returns 1; pgrx tests pass
