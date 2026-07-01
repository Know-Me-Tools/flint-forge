# Tasks — p2-c007-openapi-compiler

## Change
OpenApiCompiler: DatabaseModel → OpenAPI 3.1 JSON + GET /openapi.json route

## Status: DONE — implemented in crates/fdb-reflection/src/compilers/openapi.rs (409 lines, 11 unit tests), GET /openapi.json wired in fdb-gateway/src/main.rs:133, openapi_doc field in CompiledState
## Priority: P1 — post-MVP
## Depends On: p2-c003 (DatabaseModel, CompiledState), p2-c005 (StateManager.current())

---

## Task List

### T1 — Implement type mapping function
- [x] `fn pg_type_to_json_schema(pg_type: &str) -> serde_json::Value`
- [x] Map all Postgres types per proposal table (integer types, numeric, boolean, text, uuid, timestamp, date, jsonb, vector, fallback)
- [x] Unit test: `test_pg_type_mapping_all_variants` — verify each type maps correctly

### T2 — Implement `table_to_schema()`
- [x] `fn table_to_schema(table: &Table) -> serde_json::Value`
- [x] Returns JSON Schema object with `properties` map from `table.columns`
- [x] Include `required` array for non-nullable columns without defaults
- [x] Include `x-flint-rls-enabled: bool` extension field

### T3 — Implement filter parameter documentation
- [x] `fn table_get_params(table: &Table) -> Vec<serde_json::Value>`
- [x] For each column × operator (12 operators): produce query parameter entry with `in: query`, `required: false`, `schema: { type: string }`
- [x] Add `order` param: `?order=<column>.(asc|desc)`
- [x] Add `Range` header param: `in: header`, name: `Range`, pattern: `items=\d+-\d+`

### T4 — Implement `table_paths()`
- [x] `fn table_paths(table: &Table, schema_ref: &str) -> serde_json::Value`
- [x] Return JSON object with `get`, `post`, `patch`, `delete` keys
- [x] GET: includes filter params from T3; response 200 with array of `$ref: #/components/schemas/<schema_ref>`
- [x] POST: request body as `$ref`, response 201 with created object
- [x] PATCH: path param `id`, request body, response 200
- [x] DELETE: path param `id`, response 200 with deleted object
- [x] All operations include `security: [{ bearerAuth: [] }]`

### T5 — Implement `fn_path()`
- [x] `fn fn_path(func: &FnMeta) -> serde_json::Value`
- [x] POST operation with request body containing `func.args` as JSON Schema properties
- [x] Response 200 with `type: array`
- [x] Include `security: [{ bearerAuth: [] }]`
- [x] Include `description: "Postgres function: <schema>.<name>"`

### T6 — Implement `OpenApiCompiler::compile()`
- [x] `pub fn compile(model: &DatabaseModel) -> serde_json::Value`
- [x] Build `paths` map from tables (using `table_paths()`) + functions (using `fn_path()`)
- [x] Build `components.schemas` map from tables (using `table_to_schema()`)
- [x] Assemble top-level OpenAPI 3.1.0 document with `info`, `paths`, `components`, `security`
- [x] Export `OpenApiCompiler` from `fdb-reflection/src/lib.rs`

### T7 — Wire `GET /openapi.json` into `fdb-gateway`
- [x] Add route: `GET /openapi.json` → return `Json(state_manager.current().openapi_doc.clone())`
- [x] Add response header: `Content-Type: application/json`
- [x] Verify route is registered after `StateManager::new()` returns (initial compile complete)

### T8 — Unit tests `tests/openapi_compiler.rs`
- [x] `test_openapi_version_is_3_1_0`
- [x] `test_every_table_has_crud_paths`
- [x] `test_column_types_map_correctly` (integer, uuid, jsonb, vector)
- [x] `test_function_has_post_path`
- [x] `test_openapi_doc_is_valid_json`
- [x] `test_bearer_security_scheme_present`
- [x] `test_filter_params_documented_for_all_12_operators`

### T9 — HTTP integration test
- [x] Start `fdb-gateway` against test database
- [x] `GET /openapi.json` → 200, `Content-Type: application/json`
- [x] Parse response: `openapi == "3.1.0"` at root
- [x] Paths for all test-DB tables present

### T10 — Final verification
- [x] `cargo test -p fdb-reflection -- openapi` passes
- [x] `cargo test -p fdb-gateway` passes (openapi route)
- [x] `cargo clippy --workspace -- -D warnings` — no warnings
- [x] `cargo check --workspace` — clean build
