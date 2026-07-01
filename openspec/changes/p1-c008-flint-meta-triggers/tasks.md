# p1-c008 — Tasks

## Pre-implementation
- [ ] Research pgrx 0.18.1 event trigger API — find the correct function signature for `RETURNS event_trigger` in pgrx (check pgrx repo examples/tests/event_trigger*)
- [ ] Confirm `pg_event_trigger_ddl_commands()` SPI call pattern in pgrx

## Implementation
- [ ] Create `crates/ext-flint-meta/src/triggers.rs` module
- [ ] Implement `flint_meta_refresh_cache()` unsafe event trigger fn:
  - [ ] Query `pg_event_trigger_ddl_commands()` via SPI
  - [ ] For each object: route to correct cache table update (TABLE → cache_tables, COLUMN → cache_columns, CONSTRAINT → cache_relationships, FUNCTION → cache_functions, TYPE → cache_types, POLICY → cache_policies)
  - [ ] Skip objects in `flint_meta` and `vault` schemas (avoid self-invalidation)
  - [ ] Guard: skip if `object_identity LIKE '%ext-flint-meta%'`
  - [ ] INSERT INTO `flint_meta.schema_version` (version, ddl_tag, object_identity)
  - [ ] Call `pg_notify('meta_runtime', payload_json)` where payload = `{version, ddl_tag, object_identity}`
- [ ] Implement `flint_meta_invalidate_cache()` unsafe event trigger fn:
  - [ ] Query `pg_event_trigger_dropped_objects()` via SPI
  - [ ] DELETE from cache tables for each dropped object
  - [ ] INSERT INTO `flint_meta.schema_version`, `pg_notify`
- [ ] Implement `full_refresh()` SQL function: truncate + repopulate all cache_* from pg_catalog
- [ ] Register trigger functions and event triggers via `extension_sql!`
- [ ] Add event triggers to `sql/flint_meta.sql` (or separate SQL file with `requires`)

## Documentation
- [ ] Write `docs/contracts/meta-trigger-coverage.md`:
  - [ ] DDL events covered: `CREATE TABLE`, `ALTER TABLE`, `DROP TABLE`, `CREATE VIEW`, `CREATE FUNCTION`, `CREATE TYPE`, `CREATE POLICY`, `COMMENT ON`
  - [ ] DDL events NOT covered: `CREATE TABLE AS` (PG ≤ 15), `SELECT INTO`, nested event trigger DDL
  - [ ] Mitigation: `full_refresh()` via pg_cron every 10 minutes catches gaps

## Tests
- [ ] Write pgrx `#[pg_test]` for refresh: `CREATE TABLE test_001 (id uuid)` → assert `flint_meta.schema_version.version > 1`
- [ ] Write pgrx `#[pg_test]` for invalidate: `DROP TABLE test_001` → assert cache_tables row removed
- [ ] Write pgrx `#[pg_test]` for notify payload: verify pg_notify was called (check via SPI query on pg_listening_channels or inspect schema_version)

## Verification
- [ ] Run `cargo pgrx test -p ext-flint-meta --features pg18` — all tests pass
- [ ] GATE: `CREATE TABLE` increments schema_version; `DROP TABLE` removes from cache; pg_notify fires
