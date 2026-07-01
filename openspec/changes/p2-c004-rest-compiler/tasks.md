# Tasks — p2-c004-rest-compiler

## Change
RestCompiler: DatabaseModel → axum::Router CRUD handlers

## Status: PENDING
## Depends On: p2-c003 (IR types must exist), p2-c002 (Conn for handlers)

---

## Task List

### T1 — Create `src/compilers/handlers.rs`
- [ ] Define `extract_rls(headers: &HeaderMap) -> Result<RlsContext, RestError>`
  - Extract `Authorization: Bearer <token>` header
  - Return `RestError::Unauthorized` if absent or malformed
  - Call `fdb_auth::rls_from_bearer(token)` to get `RlsContext`

### T2 — Implement column name validation
- [ ] `validate_column<'a>(col: &'a str, table: &Table) -> Result<&'a str, RestError>`
  - Check `table.columns.iter().any(|c| c.name == col)`
  - Return `RestError::UnknownColumn(col.to_string())` if not found
- [ ] Ensure all ORDER BY and SELECT column names go through this function
- [ ] SECURITY: parameterize VALUES only; column names use allowlist, not bind params

### T3 — Implement filter parser
- [ ] `parse_filters(params: &HashMap<String, String>, table: &Table) -> Result<(String, Vec<Box<dyn ToSql>>), RestError>`
  - Parse `key=op.value` format from query params
  - Map operators to SQL: `eq` → `=`, `neq` → `!=`, `gt` → `>`, `gte` → `>=`, `lt` → `<`, `lte` → `<=`, `like` → `LIKE`, `ilike` → `ILIKE`, `in` → `= ANY($N)`, `is` → `IS NULL/NOT NULL`, `cs` → `@>`, `cd` → `<@`
  - Validate column name via `validate_column()` before use
  - Return `RestError::InvalidOperator` for unknown operator strings

### T4 — Implement pagination parser
- [ ] `parse_range(headers: &HeaderMap) -> (i64, i64)` → `(limit, offset)`
  - Parse `Range: items=<start>-<end>` header
  - Default: `(1000, 0)` when header absent
  - Compute: `limit = end - start + 1`, `offset = start`
  - Return `Content-Range: items <start>-<end>/<total>` header in responses

### T5 — Implement SELECT handler
- [ ] `async fn select(State, table: Table, Query(params), headers) -> Result<Json<Vec<Value>>, RestError>`
  - Call `extract_rls()`
  - Call `backend.acquire(&rls)`
  - Call `parse_filters()` and `parse_range()`
  - Build parameterized SELECT with WHERE clause and LIMIT/OFFSET
  - Execute on `conn.tx()`
  - Serialize rows to `Vec<serde_json::Value>`
  - Add `Content-Range` header to response

### T6 — Implement INSERT handler
- [ ] `async fn insert(State, table: Table, headers, Json(body)) -> Result<Json<Value>, RestError>`
  - Call `extract_rls()`
  - Call `backend.acquire(&rls)`
  - Extract column names from `body` JSON object keys
  - Validate each column name against `table.columns`
  - Build `INSERT INTO schema.table (col1, col2) VALUES ($1, $2) RETURNING *`
  - Execute with bound values

### T7 — Implement UPDATE handler
- [ ] `async fn update(State, table: Table, Path(id), headers, Json(body)) -> Result<Json<Value>, RestError>`
  - Call `extract_rls()`
  - Call `backend.acquire(&rls)`
  - Validate each column in body against table
  - Build `UPDATE schema.table SET col1 = $2, col2 = $3 WHERE id = $1 RETURNING *`
  - Execute with id and column values as bound params

### T8 — Implement DELETE handler
- [ ] `async fn delete(State, table: Table, Path(id), headers) -> Result<Json<Value>, RestError>`
  - Call `extract_rls()`
  - Call `backend.acquire(&rls)`
  - Build `DELETE FROM schema.table WHERE id = $1 RETURNING *`
  - Execute with id as bound param

### T9 — Implement RPC handler
- [ ] `async fn rpc(State, func: FnMeta, headers, Json(body)) -> Result<Json<Value>, RestError>`
  - Call `extract_rls()`
  - Call `backend.acquire(&rls)`
  - Match body JSON keys to `func.args` by name
  - Build `SELECT * FROM schema.fn($1, $2, ...)` with bound args
  - Execute and return result rows as JSON

### T10 — Define `RestError` and `IntoResponse`
- [ ] Define `RestError` with variants from proposal
- [ ] Implement `IntoResponse`: `UnknownColumn` + `InvalidOperator` → 400; `Unauthorized` → 401; others → 500 + tracing::error (no internal detail to client)

### T11 — Implement `RestCompiler::compile()`
- [ ] Iterate `model.tables` — register GET + POST on `/<schema>/<table>`
- [ ] Register PATCH + DELETE on `/<schema>/<table>/:id`
- [ ] Iterate `model.functions` — register POST on `/rpc/<schema>/<fn>`
- [ ] Return assembled `axum::Router`

### T12 — Integration tests `tests/rest_compiler.rs`
- [ ] `test_rest_select_with_eq_filter`
- [ ] `test_rest_select_all_filter_operators` (12 operators)
- [ ] `test_rest_insert_returns_row`
- [ ] `test_rest_patch_by_id`
- [ ] `test_rest_delete_by_id`
- [ ] `test_rest_unknown_column_returns_400`
- [ ] `test_rest_rpc_call`
- [ ] `test_rls_role_visible_in_handler`
- [ ] Mark tests `#[ignore]` if no `DATABASE_URL` env var

### T13 — Final verification
- [ ] `cargo test -p fdb-reflection -- rest` passes
- [ ] `cargo clippy --workspace -- -D warnings` — no warnings
- [ ] `cargo check --workspace` — clean build
