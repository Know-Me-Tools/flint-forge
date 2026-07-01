# p1-c010 — ext-flint-meta: AG-UI descriptor + OpenAPI functions

## Why

The `flint-reflection` compiler (Phase 2) emits an MCP tools list and AG-UI descriptor as part of the `CompiledState`. Rather than computing this purely in Rust from pg_catalog, the database itself can generate a descriptor that the Rust engine uses as a seed (and can augment). These functions make the schema self-describing in the format that AI agents and API consumers expect.

## What

### `flint_meta.agui_descriptor()` → JSONB

Returns an AG-UI-compatible tool descriptor array. Each table becomes one tool:

```json
{
  "tools": [
    {
      "name": "query_<schema>_<table>",
      "description": "<table description or generated>",
      "parameters": {
        "type": "object",
        "properties": {
          "<col_name>": {
            "type": "<json_type>",
            "description": "<column description>"
          }
        },
        "required": ["<pk_columns>"]
      }
    }
  ]
}
```

### `flint_meta.openapi()` → JSONB

Returns a partial OpenAPI 3.1 `paths` object built from cache_tables + cache_columns + cache_relationships:

```json
{
  "/rest/v1/<table>": {
    "get": {
      "summary": "Query <table>",
      "parameters": [...],
      "responses": {"200": {...}}
    },
    "post": {
      "summary": "Insert into <table>",
      "requestBody": {...},
      "responses": {"201": {...}}
    }
  }
}
```

Both functions are STABLE and read from `flint_meta.cache_*` tables populated by p1-c008/p1-c009.

## Contract

`SELECT jsonb_array_length(flint_meta.agui_descriptor()->'tools') > 0` returns true when at least one non-system table exists. `SELECT flint_meta.openapi()` returns valid JSONB. The tool names follow the convention `query_<schema>_<table>`.

## Out of scope

MCP server endpoint (p7-c007). The Rust compiler's augmentation layer (Phase 2). These functions are the DB-native seed; `flint-reflection` adds relationship traversal and RLS-aware parameter generation on top.

## Constraints

- STABLE PARALLEL SAFE — no writes, no GUC mutation
- GRANT EXECUTE TO authenticated, anon (AG-UI descriptor is public-facing)
- File size ≤ 500 lines — `src/descriptors.rs`

## Reference

- AG-UI protocol spec: `sdks/community/rust/crates/ag-ui-client` in flint-gate
- `docs/FLINT-PHASE-PLAN-REVISED.md` §Phase 1 p1-c010
- OpenAPI 3.1 specification §paths object
