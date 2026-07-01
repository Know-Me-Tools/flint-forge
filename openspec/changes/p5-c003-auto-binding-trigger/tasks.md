# p5-c003 Tasks — Auto-Binding Trigger

## Tasks

- [x] Add `flint_a2ui.auto_generate_bindings()` trigger function — migrations/0003_a2ui_triggers.sql
- [x] Add `CREATE TRIGGER a2ui_auto_bind_tables AFTER INSERT ON flint_meta.cache_tables FOR EACH ROW EXECUTE FUNCTION flint_a2ui.auto_generate_bindings()` (with DROP IF EXISTS guard for idempotency)
- [x] Add `flint_a2ui.column_type_to_component(pg_type text)` IMMUTABLE function — covers text, int, float, bool, date/timestamp, jsonb, uuid, unknown
- [x] Patch `ext-flint-meta/src/agui.rs` line 80: change protocol label from `'ag-ui/1.0'` to `'flint-forge/schema-descriptor/1.0'`
- [x] Update `test_agui_descriptor_returns_jsonb` test (line 253) to assert new protocol label
- [x] Gate test: auto-binding trigger — inserts BASE TABLE row, verifies grid+form+detail bindings + audit event — crates/fdb-gateway/tests/a2ui_trigger_test.rs
- [x] Gate test: VIEW row — verifies grid+detail only (no form binding) — same test file
- [x] Gate test: column_type_to_component — 13 type cases verified — same test file
