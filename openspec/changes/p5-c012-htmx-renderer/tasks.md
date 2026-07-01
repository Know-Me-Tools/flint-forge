# p5-c012 Tasks — Flint HTMX Renderer

## Tasks

- [ ] Add `askama` and `axum-htmx` crates to `fdb-gateway` Cargo.toml
- [ ] Create `crates/fdb-gateway/templates/` directory with base.html (HTMX + DaisyUI CDN)
- [ ] Implement `HtmlTemplate<T>` Axum response wrapper (wraps Askama template as HTML response)
- [ ] Implement `crates/fdb-gateway/src/routes/htmx.rs` with route handlers for all component slugs
- [ ] Create Askama template for each of the 63 Flint components (one `.html` file per slug)
- [ ] Implement SSE route: `GET /htmx/stream/:surface_id` — AG-UI events → HTMX OOB swap fragments
- [ ] Implement admin UI templates: registry management, application management, design systems
- [ ] Register HTMX routes in `fdb-gateway/src/main.rs` under `/htmx/*` prefix
- [ ] Gate test: render all 63 component templates with test props → validate HTML structure
- [ ] Gate test: SSE streaming test with mock AG-UI events
- [ ] Gate test: W3C HTML validation on all rendered templates
- [ ] Document: add `/htmx` section to fdb-gateway API docs
