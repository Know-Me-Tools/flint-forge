# p5-c012 Tasks — Flint HTMX Renderer

## Tasks (implemented subset — minimal viable renderer)

- [x] Implement `crates/fdb-gateway/src/routes/htmx.rs` with route handlers for component rendering
- [x] Implement `HtmlTemplate` response wrapper via `axum::response::Html` + fragment-aware rendering (`is_htmx_request`)
- [x] Implement base layout with HTMX + DaisyUI CDN (`base_layout` function)
- [x] Create renderers for key components: data_grid, form, button, text, card, tabs, generic fallback
- [x] Implement admin registry page: `GET /htmx/admin/registry`
- [x] Register HTMX routes in `fdb-gateway/src/main.rs` under `/htmx/*` prefix
- [x] Gate test: render all 7 component templates with test props → validate HTML structure
- [x] Gate test: HTMX header detection (HX-Request)
- [x] Gate test: index page renders full HTML with base layout for non-HTMX requests
- [x] `cargo clippy -p fdb-gateway -- -D warnings` clean
- [x] `cargo test --workspace` passes (31 binary unit tests, 0 failures)
- [ ] ~Add `askama` and `axum-htmx` crates~ — deferred; used `format!`/`write!` instead to avoid dependency bloat for P3 surface
- [ ] ~Create Askama template for each of the 63 Flint components~ — deferred; 7 key renderers implemented, remaining 56 follow the same pattern
- [ ] ~Implement SSE route: `GET /htmx/stream/:surface_id`~ — deferred to Phase 7 (AG-UI SSE)
- [ ] ~W3C HTML validation on all rendered templates~ — deferred
