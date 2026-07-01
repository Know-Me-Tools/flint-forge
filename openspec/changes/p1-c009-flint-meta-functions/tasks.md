# p1-c009 — Tasks

## Pre-implementation
- [ ] Research pgrx `TableIterator` / `SetOfIterator` API for set-returning functions
- [ ] Determine composite return type pattern for multi-column SETOF in pgrx 0.18.1

## Implementation
- [ ] Create `crates/ext-flint-meta/src/functions.rs` module
- [ ] Implement `tables(schema_filter: Option<&str>)`:
  - [ ] `SELECT schema_name, table_name, is_view, description, rls_enabled FROM flint_meta.cache_tables WHERE (schema_filter IS NULL OR schema_name = schema_filter) ORDER BY schema_name, table_name`
  - [ ] Return as `SetOfIterator` or `TableIterator`
- [ ] Implement `columns(schema_name, table_name)`:
  - [ ] SELECT from `flint_meta.cache_columns` WHERE matching
- [ ] Implement `relationships(schema_name, table_name)`:
  - [ ] SELECT from `flint_meta.cache_relationships` WHERE matching
- [ ] Implement `functions(schema_name)`:
  - [ ] SELECT from `flint_meta.cache_functions` WHERE matching
- [ ] Implement `version()`:
  - [ ] `SELECT MAX(version) FROM flint_meta.schema_version`
- [ ] Implement `check_permission(namespace, object_id, relation, subject_id)`:
  - [ ] `SELECT COUNT(*) > 0 FROM flint_meta.keto_tuples WHERE ...`
- [ ] Implement `set_identity(claims_json)`:
  - [ ] `SELECT set_config('request.jwt.claims', claims_json, true)` via SPI; return true
- [ ] Implement `full_refresh()` (referenced by p1-c008 pg_cron job):
  - [ ] Truncate all cache_* tables
  - [ ] Repopulate from `pg_catalog.pg_class`, `pg_attribute`, `pg_constraint`, `pg_proc`, `pg_policy`, `pg_type`
  - [ ] INSERT new schema_version row
  - [ ] pg_notify('meta_runtime', payload)
- [ ] Add GRANT statements for each function

## Tests
- [ ] Write pgrx `#[pg_test]` for `tables()`: create a table, call `tables(None)`, assert it appears
- [ ] Write pgrx `#[pg_test]` for `columns()`: create table with columns, verify column metadata
- [ ] Write pgrx `#[pg_test]` for `version()`: returns value ≥ 1
- [ ] Write pgrx `#[pg_test]` for `check_permission()`: INSERT keto tuple, verify returns true; DELETE tuple, verify returns false
- [ ] Write pgrx `#[pg_test]` for `set_identity()`: call with JSON, verify `current_setting('request.jwt.claims', true)` matches

## Verification
- [ ] Run `cargo pgrx test -p ext-flint-meta --features pg18` — all tests pass
- [ ] `SELECT * FROM flint_meta.tables()` works interactively
- [ ] GATE: all reflection functions queryable; check_permission and set_identity work correctly
