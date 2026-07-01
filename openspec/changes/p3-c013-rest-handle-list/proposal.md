# p3-c013 — REST handle_list with 12 Filter Operators + is_safe_identifier

## Change ID
`p3-c013-rest-handle-list`

## Phase
`p3-auth-rls-keto`

## Goal Mapping
**G3 (list)** — full RLS CRUD handler body for `handle_list` in `RestCompiler`.

## Problem
`fdb-reflection/src/compilers/rest.rs` registers routes and implements
`handle_rpc` fully (pgvector dispatch). `handle_list` returns `todo!()`.

## Scope
- Implement `is_safe_identifier(name: &str) -> bool` in `forge-domain` or
  `fdb-reflection` (whichever already owns table/column validation). The
  function is the **single chokepoint** for every identifier interpolated
  into SQL. Pattern: ASCII alphanumeric + underscore, must start with a
  letter or underscore, max 63 chars (Postgres NAMEDATALEN-1), and not a
  reserved keyword (small denylist or `tokio_postgres` quoting check).
- Implement `handle_list`:
  - Parse query params into filters using 12 operators:
    `eq`, `neq`, `gt`, `gte`, `lt`, `lte`, `like`, `ilike`, `in`, `is`, `cs`, `cd`
  - Validate EVERY column name with `is_safe_identifier()` before interpolation
  - Build `SELECT ... WHERE ... ORDER BY ... LIMIT ... OFFSET ...` with
    parameterized values (`$1`, `$2`, …)
  - Parse `Range: rows=<start>-<end>` header → `LIMIT (end-start+1) OFFSET start`
  - Emit `Content-Range: rows <start>-<end>/<total>` response header
  - RLS context is already set by `fdb-postgres::acquire()` (6 GUCs) — no extra GUC work
- Split into helper functions; if `rest.rs` approaches 500 lines, extract a
  `filters/` directory module.

## Out of Scope
- `handle_insert`, `handle_update`, `handle_delete` (c014).
- Gate test `test_rest_select_with_eq_filter` (c015) — this change makes it possible.

## Acceptance Criteria
- [ ] `is_safe_identifier()` exists with unit tests covering: valid names, SQL-injection attempts, reserved words, over-length names
- [ ] `handle_list` returns real rows for a known table with no filter
- [ ] All 12 filter operators produce correct SQL (verifiable via test)
- [ ] `Range` header drives LIMIT/OFFSET; `Content-Range` echoed
- [ ] No identifier reaches SQL without passing `is_safe_identifier()`
- [ ] `rest.rs` ≤ 500 lines (split if needed)
- [ ] `cargo check` + clippy + `cargo test -p fdb-reflection` green
