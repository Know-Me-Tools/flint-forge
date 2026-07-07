# p5-c006 Tasks — A2UI REST API

## Tasks

- [x] Add `fdb-gateway/src/routes/a2ui.rs` with all 8 endpoints
- [x] Implement `GET /a2ui/v1/components` — calls `flint_a2ui.resolve_components(app_id, jwt)` and returns array
- [x] Implement `GET /a2ui/v1/components/:slug` — single component lookup
- [x] Implement `POST /a2ui/v1/components/search` — hybrid BM25 + vector search (requires p5-c004)
- [x] Implement `GET /a2ui/v1/components/bindings/:schema/:table` — returns auto-generated bindings
- [x] Implement `GET /a2ui/v1/catalog/:catalog_id` — serves A2UI catalog JSON Schema (OQ-11 resolution)
- [x] Implement `POST /a2ui/v1/surfaces/assemble` — delegates to p5-c007 assembler
- [x] Wire routes into `fdb-gateway/src/main.rs` Axum router
- [x] Add JWT extraction middleware to all `/a2ui/v1/*` routes
- [x] Gate test: `GET /a2ui/v1/components` returns base components for valid JWT
- [x] Gate test: `GET /a2ui/v1/catalog/flint-base/1.0` returns valid A2UI catalog JSON Schema
- [x] Gate test: unauthenticated request to `/a2ui/v1/components` returns 401
