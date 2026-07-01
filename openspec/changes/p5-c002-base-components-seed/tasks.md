# p5-c002 Tasks — Base Components Seed

## Tasks

- [x] Create `scripts/seed_a2ui_components.sql` with all 55+ base component definitions
- [x] Layout category (8): container, row, column, grid, stack, divider, spacer, scroll-area
- [x] Data-display category (12): data-grid, data-table, text, badge, tag, avatar, stat-card, timeline, code-block, json-viewer, list, detail-view
- [x] Input category (14): form, text-input, number-input, select, multi-select, date-picker, checkbox, radio, toggle, textarea, file-upload, search-input, color-picker, slider
- [x] Action category (6): button, action-bar, dropdown-menu, context-menu, fab, link
- [x] Navigation category (6): nav-bar, sidebar, tabs, breadcrumb, pagination, stepper
- [x] Feedback category (8): alert, toast, modal, dialog, loading-spinner, progress-bar, empty-state, error-boundary
- [x] System category (1): flint-meta-schema (self-registration of flint_meta schema descriptor)
- [x] All inserts use `ON CONFLICT (slug) DO UPDATE` for idempotency (flint-meta-schema uses DO NOTHING — intentional)
- [x] Each component has: slug, category, primitive_type, schema (valid JSON Schema), is_base=true, description, at least one usage_example
- [x] Wire `scripts/seed_a2ui_components.sql` to run after migration via `include_str!` + `sqlx::raw_sql` in fdb-gateway/src/main.rs
- [x] Gate tests: crates/fdb-gateway/tests/a2ui_seed_test.rs — COUNT ≥ 50, all 7 categories, key components, flint-meta-schema, valid schemas
