# p1-c009 — ext-flint-meta: SQL-callable reflection functions

## Why

`flint-reflection` (Phase 2) needs a clean SQL interface to the cache tables rather than querying `pg_catalog` directly. These functions also serve the AG-UI descriptor (p1-c010) and any DDL-time inspection tools. `check_permission()` and `set_identity()` are used by Phase 2's RLS context assembly.

## What

Implement as pgrx `#[pg_extern]` functions in `src/functions.rs`:

### Read-only reflection functions (STABLE, PARALLEL SAFE)

```rust
// Returns SETOF ROW(schema_name, table_name, is_view, description, rls_enabled)
#[pg_extern(stable, parallel_safe)]
fn tables(schema_filter: Option<&str>) -> SetOfIterator<...>

// Returns SETOF ROW(column_name, data_type, is_nullable, is_pk, is_unique, default_expr, description)
#[pg_extern(stable, parallel_safe)]
fn columns(schema_name: &str, table_name: &str) -> SetOfIterator<...>

// Returns SETOF ROW(column_name, foreign_schema, foreign_table, foreign_column, constraint_name)
#[pg_extern(stable, parallel_safe)]
fn relationships(schema_name: &str, table_name: &str) -> SetOfIterator<...>

// Returns SETOF ROW(function_name, argument_types, return_type, is_stable, description)
#[pg_extern(stable, parallel_safe)]
fn functions(schema_name: &str) -> SetOfIterator<...>

// Returns current schema version number
#[pg_extern(stable, parallel_safe)]
fn version() -> i64
```

### Permission check (VOLATILE — reads keto_tuples)

```rust
#[pg_extern(volatile, parallel_safe)]
fn check_permission(
    namespace: &str,
    object_id: &str,
    relation: &str,
    subject_id: &str,
) -> bool
// SELECT COUNT(*) > 0 FROM flint_meta.keto_tuples WHERE ...
```

### Identity setter (VOLATILE — sets GUC)

```rust
#[pg_extern(volatile, parallel_safe)]
fn set_identity(claims_json: &str) -> bool
// set_config('request.jwt.claims', claims_json, true)
// Returns true on success
```

All functions are in the `flint_meta` SQL schema.

## Contract

`SELECT * FROM flint_meta.tables()` returns a row for every non-system table in the database. `SELECT flint_meta.version()` returns the current schema version (≥ 1). `SELECT flint_meta.check_permission('app', 'doc-1', 'view', 'user-1')` returns false when no matching tuple exists.

## Constraints

- `tables()`, `columns()`, `relationships()`, `functions()` are `STABLE PARALLEL SAFE` — no side effects
- `check_permission()` and `set_identity()` are `VOLATILE` — they read/write state
- No `unwrap()` / `expect()` — use `error!()` for fatal conditions, return empty iterators for no-match
- GRANT: `EXECUTE ON flint_meta.tables, columns, relationships, functions, version TO authenticated, anon`; `check_permission, set_identity TO service_role` only

## Reference

- pgrx `SetOfIterator` / `TableIterator` for set-returning functions
- `docs/FLINT-PHASE-PLAN-REVISED.md` §Phase 1 p1-c009
