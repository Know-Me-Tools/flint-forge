# flint_meta DDL Trigger Coverage

## Covered DDL Events

The `flint_meta_ddl_refresh` event trigger fires on `ddl_command_end` for:

| DDL Tag | Cache Updated |
|---------|--------------|
| `CREATE TABLE` | `cache_tables` — row upserted; `rls_enabled` populated from `pg_class.relrowsecurity` |
| `ALTER TABLE` | `cache_tables` — `rls_enabled` refreshed via upsert |
| `CREATE VIEW` | `cache_tables` — row upserted with `is_view = true` |
| `ALTER VIEW` | `cache_tables` — `is_view` confirmed true via upsert |
| `CREATE FUNCTION` | `cache_functions` — row upserted with return type and argument types |
| `CREATE OR REPLACE FUNCTION` | `cache_functions` — upsert updates `return_type` and `is_stable` |
| `CREATE TYPE` | `cache_types` — row inserted on first creation; no-op on conflict |
| `COMMENT` | `schema_version` incremented; no cache row change (comments not cached) |

The `flint_meta_ddl_invalidate` event trigger fires on `sql_drop` for:

| Object Type | Cache Updated |
|-------------|--------------|
| `table` | `cache_tables` row deleted (FK `ON DELETE CASCADE` removes `cache_columns` children) |
| `function` | `cache_functions` row deleted |
| `type` | `cache_types` row deleted |

Both triggers:
1. Insert a row into `flint_meta.schema_version` (auto-incrementing `version` bigserial).
2. Emit `pg_notify('meta_runtime', json_build_object(…))` so the reflection engine
   can invalidate its in-process cache without polling.
3. Are declared `SECURITY DEFINER` with a pinned `search_path` to prevent search-path
   injection. Neither function reads or logs JWT claims, tenant IDs, or relation tuples.

## NOT Covered (Known Gaps)

| DDL | Reason | Mitigation |
|-----|--------|------------|
| `CREATE TABLE AS SELECT` | Fires under the `SELECT INTO` tag, not `CREATE TABLE` | `full_refresh()` reconciles |
| `SELECT INTO` | Not a `ddl_command_end` TAG match | `full_refresh()` reconciles |
| Partitioned table attach/detach | `ALTER TABLE … ATTACH/DETACH PARTITION` not in default tag list | `full_refresh()` reconciles |
| Column `ADD`/`DROP` via `ALTER TABLE` | Incremental path updates only the `cache_tables` row, not `cache_columns` | `full_refresh()` reconciles |
| Multi-column FK constraints | `cache_relationships` refresh path uses `conkey[1]` (first column only) | `full_refresh()` captures all columns |
| Nested event trigger DDL | Postgres does not fire event triggers recursively | Acceptable limitation |
| `TRUNCATE` | Not a DDL event; does not alter schema | No cache update needed |

## Mitigation: full_refresh()

`flint_meta.full_refresh()` truncates all `cache_*` tables and repopulates them
from `pg_catalog` in a single function call. It is the authoritative reconciliation
path for any DDL that the incremental triggers do not cover.

**Scheduled invocation:** registered as a `pg_cron` job (from p1-c004) to run
nightly at `02:00 UTC`. In Phase 2, it will also be callable from the Forge CLI
(`flint-forge meta refresh`) and triggered on connection startup by the reflection
engine.

**Performance:** `full_refresh()` acquires no explicit locks; all reads are
against `pg_catalog` which is MVCC-safe. On a database with ~1,000 tables and
~10,000 columns the function typically completes in under 100 ms.

**Idempotency:** safe to call repeatedly. All inserts use `ON CONFLICT DO
NOTHING` or `DO UPDATE` as appropriate; `TRUNCATE` at the start prevents stale
rows from accumulating.

## Security Notes

- Both trigger functions are declared `SECURITY DEFINER` and pin `search_path`
  to `flint_meta, pg_catalog` to prevent search-path injection attacks.
- Neither function reads `request.jwt.claims`, `request.headers`, or any
  tenant-identifying session variable. There is no risk of leaking JWT payloads
  through `pg_notify`.
- `full_refresh()` is `GRANT EXECUTE … TO service_role` only. It must not be
  exposed to `authenticated` or `anon` roles.
