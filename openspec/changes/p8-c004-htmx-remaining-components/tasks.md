# p8-c004 Tasks — HTMX Remaining 48 Renderers

## Tasks

- [ ] **Split htmx.rs into module directory FIRST:** Create `routes/htmx/mod.rs`, `routes/htmx/renderers.rs`, `routes/htmx/admin.rs`; move existing code; update `routes/mod.rs`
- [ ] **Input renderers (13):** `text-input`, `number-input`, `select`, `multi-select`, `date-picker`, `checkbox`, `radio`, `toggle`, `textarea`, `file-upload`, `search-input`, `color-picker`, `slider`
- [ ] **Action renderers (5):** `action-bar`, `dropdown-menu`, `context-menu`, `fab`, `link`
- [ ] **Navigation renderers (5):** `nav-bar`, `sidebar`, `breadcrumb`, `pagination`, `stepper`
- [ ] **Feedback renderers (8):** `alert`, `toast`, `modal`, `dialog`, `loading-spinner`, `progress-bar`, `empty-state`, `error-boundary`
- [ ] **Data-display renderers (10):** `data-table`, `badge`, `tag`, `avatar`, `stat-card`, `timeline`, `code-block`, `json-viewer`, `list`, `detail-view`
- [ ] **Layout renderers (7):** `container`, `row`, `column`, `grid`, `stack`, `divider`, `spacer`, `scroll-area`
- [ ] Add all 48 new slugs to the `render_component_html()` dispatch match in `mod.rs`
- [ ] `cargo clippy -p fdb-gateway -- -D warnings` clean
- [ ] `cargo test -p fdb-gateway` passes — add unit tests for at least 5 new renderers
- [ ] All files in `routes/htmx/` stay under 500 lines
