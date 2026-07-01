# p1-c010 — Tasks

## Pre-implementation
- [ ] Review AG-UI tool descriptor schema (from flint-gate `ag-ui-client` crate or protocol spec) — confirm `name`, `description`, `parameters` fields
- [ ] Decide PG type → JSON Schema type mapping (text→string, integer→integer, boolean→boolean, timestamptz→string/date-time, uuid→string/uuid, jsonb→object, etc.)

## Implementation
- [ ] Create `crates/ext-flint-meta/src/descriptors.rs` module
- [ ] Implement `agui_descriptor()` → JSONB:
  - [ ] For each row in `flint_meta.cache_tables(schema_filter := 'public')`:
    - [ ] Build tool name: `format!("query_{}_{}", schema, table)`
    - [ ] Build `parameters.properties` from `flint_meta.cache_columns` for that table
    - [ ] Build `required` list from columns WHERE `is_pk = true`
    - [ ] Use table/column `description` where available
  - [ ] Return `jsonb_build_object('tools', jsonb_agg(tool))` equivalent
- [ ] Implement `openapi()` → JSONB:
  - [ ] For each table: build a `paths` entry with GET + POST operations
  - [ ] GET parameters: PK and filter columns as query params
  - [ ] POST requestBody: schema built from non-generated columns
  - [ ] Relationships → `$ref` links between schemas
- [ ] Add GRANT statements

## Tests
- [ ] Write pgrx `#[pg_test]` for `agui_descriptor()`: create test table, call descriptor, assert tool exists in output
- [ ] Write pgrx `#[pg_test]` for `openapi()`: create test table, call openapi, assert path exists
- [ ] Verify type mapping: uuid column maps to `{type: "string", format: "uuid"}`

## Verification
- [ ] Run `cargo pgrx test -p ext-flint-meta --features pg18` — all tests pass
- [ ] `SELECT flint_meta.agui_descriptor()` returns valid JSONB with tools array
- [ ] GATE: tools array non-empty when tables exist; openapi() returns paths object
