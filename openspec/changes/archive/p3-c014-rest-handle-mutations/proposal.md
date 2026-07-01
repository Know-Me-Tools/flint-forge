# p3-c014 — REST handle_insert / handle_update / handle_delete

## Change ID
`p3-c014-rest-handle-mutations`

## Phase
`p3-auth-rls-keto`

## Goal Mapping
**G3 (mutations)** — completes CRUD surface started in c013.

## Problem
`handle_insert`, `handle_update`, `handle_delete` all return `todo!()`.

## Scope
- `handle_insert`:
  - Parse JSON body into column/value pairs
  - Validate column names with `is_safe_identifier()` (c013 utility)
  - `INSERT INTO <tbl> (<cols>) VALUES ($1, $2, …) RETURNING *`
  - `201 Created` with `Location: /rest/<tbl>/<pk>` header
- `handle_update`:
  - Parse query filter (reuse c013 operator dispatch)
  - Parse JSON body for SET values
  - `UPDATE <tbl> SET <col>=$1, … WHERE <filter> RETURNING *`
  - `200 OK` with updated row(s) or `204 No Content` if no RETURNING
- `handle_delete`:
  - Parse query filter (reuse c013 operator dispatch)
  - `DELETE FROM <tbl> WHERE <filter> RETURNING *`
  - `204 No Content`
- All mutations MUST call `KetoCheck::check()` (c011) and `Pep::check()`
  (c012) before executing SQL. Failures map to 403.
- Reuse parameterization and identifier safety from c013 verbatim.

## Out of Scope
- Gate tests (c015).
- RPC handlers (already implemented).

## Acceptance Criteria
- [ ] All three handlers return real SQL results
- [ ] 201/200/204 status codes correct per handler
- [ ] `Location` header on insert; correct `Content-Range` semantics
- [ ] Keto + Cedar gates invoked before any mutation
- [ ] No identifier reaches SQL without `is_safe_identifier()`
- [ ] No `unwrap()`/`expect()` in library code
- [ ] `cargo check` + clippy + `cargo test -p fdb-reflection` green
