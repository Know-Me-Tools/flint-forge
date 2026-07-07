# p5-c006 — A2UI Registry REST API

**Phase:** 5 — Flint A2UI Component Registry  
**Priority:** P1  
**Depends on:** p5-c001, p5-c005, Phase 2 (fdb-gateway Axum router)  
**Blocks:** p5-c008 (MCP tools need REST endpoints to delegate to)

---

## What this change delivers

REST endpoints in `fdb-gateway` for the A2UI component registry. All endpoints are JWT-gated (flint-gate auth) and RLS-enforced.

### Endpoints

```
GET    /a2ui/v1/components                  # list components (permission-filtered)
GET    /a2ui/v1/components/:slug            # get component by slug
POST   /a2ui/v1/components/search           # text + semantic search
GET    /a2ui/v1/components/bindings/:schema/:table  # get auto-generated bindings for a table

GET    /a2ui/v1/applications                # list applications (admin only)
GET    /a2ui/v1/applications/:id            # get application

GET    /a2ui/v1/catalog/:catalog_id         # serve A2UI catalog as JSON Schema (for CopilotKit)
                                            # catalog_id: e.g. "flint-base/1.0"

POST   /a2ui/v1/surfaces/assemble           # assemble A2UI surface from event context
```

### Catalog endpoint (for CopilotKit integration)

The `GET /a2ui/v1/catalog/:catalog_id` endpoint serves the catalog in a format compatible with CopilotKit's `<CopilotKit a2ui={{ catalog }}>` prop. This resolves OQ-11.

The catalog format follows the A2UI v0.9.1 catalog definition schema: a JSON Schema where `definitions` contains each component type.

```json
{
    "$schema": "https://a2ui.org/schemas/catalog/v0.9.1",
    "catalogId": "https://forge.example.com/a2ui/v1/catalog/flint-base/1.0",
    "name": "Flint Base Catalog",
    "version": "1.0.0",
    "definitions": {
        "DataGrid": { ... },
        "TextInput": { ... }
    }
}
```
