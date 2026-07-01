# p5-c006 Tasks — A2UI REST API

## Tasks

- [ ] Add `fdb-gateway/src/routes/a2ui.rs` with all 8 endpoints
- [ ] Implement `GET /a2ui/v1/components` — calls `flint_a2ui.resolve_components(app_id, jwt)` and returns array
- [ ] Implement `GET /a2ui/v1/components/:slug` — single component lookup
- [ ] Implement `POST /a2ui/v1/components/search` — hybrid BM25 + vector search (requires p5-c004)
- [ ] Implement `GET /a2ui/v1/components/bindings/:schema/:table` — returns auto-generated bindings
- [ ] Implement `GET /a2ui/v1/catalog/:catalog_id` — serves A2UI catalog JSON Schema (OQ-11 resolution)
- [ ] Implement `POST /a2ui/v1/surfaces/assemble` — delegates to p5-c007 assembler
- [ ] Wire routes into `fdb-gateway/src/main.rs` Axum router
- [ ] Add JWT extraction middleware to all `/a2ui/v1/*` routes
- [ ] Gate test: `GET /a2ui/v1/components` returns base components for valid JWT
- [ ] Gate test: `GET /a2ui/v1/catalog/flint-base/1.0` returns valid A2UI catalog JSON Schema
- [ ] Gate test: unauthenticated request to `/a2ui/v1/components` returns 401
