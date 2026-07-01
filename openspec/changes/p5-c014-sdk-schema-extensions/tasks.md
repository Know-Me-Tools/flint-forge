# p5-c014 Tasks — SDK Component Override + Design Token Schema Extensions

## Tasks

- [x] Create `migrations/0004_flint_a2ui_sdk_extensions.sql` (note: c003 used 0003, so this is 0004)
- [x] Add `renderers`, `react_pkg`, `flutter_pkg`, `htmx_template` columns to `flint_a2ui.components` (IF NOT EXISTS)
- [x] Create `flint_a2ui.component_overrides` table with correct RLS policy (R2 fix: uses `application_id`, not `app_id`, from role_assignments; joins roles.slug = 'app-admin')
- [x] Add `source_format`, `source_content`, `imported_at`, `token_schema_version` to `flint_a2ui.design_systems`
- [x] Implement `flint_a2ui.resolve_components_with_overrides(p_application_id, p_jwt_claims, p_design_system_id)` SECURITY DEFINER SQL function
- [x] Add `ResolvedComponent`, `Renderers`, `DesignToken`, `DesignTokenMap` Rust types to `crates/fdb-app/src/a2ui/types.rs`
- [x] Create `crates/fdb-app/src/a2ui/mod.rs` and add `pub mod a2ui` to `fdb-app/src/lib.rs`
- [x] Add `serde = { workspace = true }` to `fdb-app/Cargo.toml`
- [x] `cargo check --workspace` passes
- [ ] Update `GET /a2ui/v1/catalog/:id` endpoint — deferred: endpoint not yet created (p5-c006)
- [ ] Update base components seed to set `renderers` column — not needed; column has default `{"react": true, "flutter": true, "htmx": true}`

**Notes:**
- Migration number adjusted to 0004 because c003 created 0003_a2ui_triggers.sql
- R2 spec defect fixed: `role_assignments.application_id` (not `app_id`), joined to `roles.slug` (not `role_name`)
