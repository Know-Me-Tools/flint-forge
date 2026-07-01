# p1-c008 — ext-flint-meta: DDL event triggers → version increment → NOTIFY

## Why

The `flint-reflection` hot-swap loop (Phase 2) listens on `meta_runtime` for schema changes. The event triggers in this change are the emitters: every DDL command that affects the schema fires `refresh_cache()`, increments the version counter, and sends a `pg_notify` payload that the Rust `StateManager` receives to trigger recompilation.

## What

- Add two pgrx event trigger functions to `ext-flint-meta`:

### `flint_meta_refresh_cache()` — on `ddl_command_end`

```rust
// src/triggers.rs
#[pg_extern]
unsafe fn flint_meta_refresh_cache() -> pg_sys::Datum {
    // 1. Call pg_event_trigger_ddl_commands() → get affected objects
    // 2. For each object: update the relevant cache_* table via pg_catalog queries
    // 3. INSERT INTO flint_meta.schema_version: version = MAX(version) + 1
    // 4. pg_notify('meta_runtime', json!({version, ddl_tag, object_identity}).to_string())
    pg_sys::Datum::null()
}
```

### `flint_meta_invalidate_cache()` — on `sql_drop`

```rust
#[pg_extern]
unsafe fn flint_meta_invalidate_cache() -> pg_sys::Datum {
    // 1. Call pg_event_trigger_dropped_objects()
    // 2. DELETE from relevant cache_* tables for each dropped object
    // 3. INSERT INTO flint_meta.schema_version: version++
    // 4. pg_notify('meta_runtime', payload)
    pg_sys::Datum::null()
}
```

- Register event triggers via `extension_sql!`:

```sql
CREATE OR REPLACE FUNCTION flint_meta.refresh_cache_trigger()
RETURNS event_trigger LANGUAGE C AS '$libdir/ext_flint_meta', 'flint_meta_refresh_cache';

CREATE EVENT TRIGGER flint_meta_ddl_refresh
ON ddl_command_end EXECUTE FUNCTION flint_meta.refresh_cache_trigger();

CREATE OR REPLACE FUNCTION flint_meta.invalidate_cache_trigger()
RETURNS event_trigger LANGUAGE C AS '$libdir/ext_flint_meta', 'flint_meta_invalidate_cache';

CREATE EVENT TRIGGER flint_meta_ddl_invalidate
ON sql_drop EXECUTE FUNCTION flint_meta.invalidate_cache_trigger();
```

- Add `flint_meta.full_refresh()` SQL function: truncates all cache_* tables and repopulates from pg_catalog (used by pg_cron safety net and on LISTEN reconnect)

- Write `docs/contracts/meta-trigger-coverage.md` documenting known DDL gaps

## Contract

After `CREATE TABLE public.test_001 (id uuid)`, `SELECT MAX(version) FROM flint_meta.schema_version` returns a value > 1, and `pg_notify('meta_runtime', ...)` was called with a payload containing the version number and `ddl_tag = 'CREATE TABLE'`.

## Constraints

- pgrx event triggers require `unsafe extern "C-unwind" fn` + `RETURNS event_trigger` return type — review pgrx 0.18.1 event trigger API carefully; the function signature differs from regular `#[pg_extern]` functions
- `pg_event_trigger_ddl_commands()` and `pg_event_trigger_dropped_objects()` are SPI functions — call via `Spi::run()` or `pgrx::spi`
- Do not fire `pg_notify` if the DDL is inside a `CREATE EXTENSION` for our own extension (guard against infinite bootstrap loops)
- File size ≤ 500 lines — `src/triggers.rs` is its own module

## Reference

- pgrx event trigger API: see pgrx examples/tests for event trigger pattern
- `docs/FLINT-PHASE-PLAN-REVISED.md` §Phase 1 p1-c008 (trigger spec, DDL coverage gaps)
- PostgreSQL docs: `pg_event_trigger_ddl_commands()`, `pg_event_trigger_dropped_objects()`
