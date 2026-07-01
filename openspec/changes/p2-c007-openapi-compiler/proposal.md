# p2-c007 — OpenApiCompiler: DatabaseModel → OpenAPI 3.1 JSON

## Change ID
`p2-c007-openapi-compiler`

## Phase
`p2-quarry-reflection-engine`

## Priority
P1 — post-MVP; does not block REST gate but required for Phase 3 GraphQL introspection merge

## Problem Statement

`compilers/openapi.rs` in `fdb-reflection` is a stub. `GET /openapi.json` in
`fdb-gateway` returns nothing. The `CompiledState.openapi_doc` field is
populated by this compiler but currently holds an empty `serde_json::Value`.

SDK clients, MCP tools, and the Phase 3 introspection merge all consume
the OpenAPI document. Without it, third-party tooling cannot auto-generate
clients, and the Quarry service is undiscoverable.

## Scope

### In Scope
- `OpenApiCompiler::compile(model: &DatabaseModel) -> serde_json::Value` in
  `fdb-reflection/src/compilers/openapi.rs`
- OpenAPI 3.1.0 document structure
- One path entry per table × HTTP method (GET/POST/PATCH/DELETE)
- `/rpc/<schema>/<fn>` path per exposed Postgres function
- Schema components from `ColumnMeta` (Postgres type → JSON Schema type mapping)
- Filter query parameters documented on GET paths (all 12 filter operators)
- Range header pagination documented on GET paths
- `fdb-gateway` route: `GET /openapi.json` returning `CompiledState.openapi_doc`
- `utoipa` for OpenAPI doc builder (or hand-rolled `serde_json::Value` if simpler)

### Out of Scope
- GraphQL SDL (Phase 3)
- MCP tool descriptions (Phase 7)
- UI for browsing the API (Phase 5)
- Authentication flows in OpenAPI (beyond Bearer token note in security schemes)

## Design

### Postgres Type → JSON Schema Mapping

| Postgres Type | JSON Schema Type |
|---|---|
| `integer`, `int4`, `int8`, `bigint` | `{ "type": "integer" }` |
| `numeric`, `float4`, `float8`, `real`, `double precision` | `{ "type": "number" }` |
| `boolean`, `bool` | `{ "type": "boolean" }` |
| `text`, `varchar`, `char`, `name` | `{ "type": "string" }` |
| `uuid` | `{ "type": "string", "format": "uuid" }` |
| `timestamp`, `timestamptz` | `{ "type": "string", "format": "date-time" }` |
| `date` | `{ "type": "string", "format": "date" }` |
| `jsonb`, `json` | `{}` (any) |
| `vector(N)` | `{ "type": "array", "items": { "type": "number" }, "description": "pgvector embedding" }` |
| anything else | `{ "type": "string", "description": "Postgres type: <pg_type>" }` |

### Document Structure

```rust
pub struct OpenApiCompiler;

impl OpenApiCompiler {
    pub fn compile(model: &DatabaseModel) -> serde_json::Value {
        let mut paths = serde_json::Map::new();
        let mut components_schemas = serde_json::Map::new();

        for table in &model.tables {
            let schema_name = format!("{}_{}", table.schema, table.name);
            components_schemas.insert(schema_name.clone(), table_to_schema(table));

            // GET /<schema>/<table>
            paths.insert(
                format!("/{}/{}", table.schema, table.name),
                table_paths(table, &schema_name),
            );
        }

        for func in &model.functions {
            paths.insert(
                format!("/rpc/{}/{}", func.schema, func.name),
                fn_path(func),
            );
        }

        serde_json::json!({
            "openapi": "3.1.0",
            "info": {
                "title": "Flint Quarry REST API",
                "version": format!("{}", model.version),
                "description": "Auto-generated from live database schema via flint_meta"
            },
            "paths": paths,
            "components": {
                "schemas": components_schemas,
                "securitySchemes": {
                    "bearerAuth": {
                        "type": "http",
                        "scheme": "bearer",
                        "bearerFormat": "JWT",
                        "description": "flint-gate JWT; role claim required on authenticated routes"
                    }
                }
            },
            "security": [{ "bearerAuth": [] }]
        })
    }
}
```

### Filter Parameter Documentation (GET paths)

Each GET path includes query parameter entries for every column × operator
combination, documented as optional string parameters:

```
?<column>=eq.<value>
?<column>=gt.<value>
... (all 12 operators)
```

Plus:
- `order` parameter: `<column>.<asc|desc>`
- `Range` header for pagination

### fdb-gateway Route

```rust
// crates/fdb-gateway/src/main.rs
.route("/openapi.json", get(move |State(sm): State<Arc<StateManager>>| async move {
    let state = sm.current();
    Json(state.openapi_doc.clone())
}))
```

### utoipa vs Hand-Rolled

Phase 2 uses hand-rolled `serde_json::Value` construction (simpler, fewer
deps). `utoipa` is reserved for Phase 3 when the GraphQL introspection merge
requires more structured merging of SDL types into the OpenAPI doc.

If `utoipa` is added later, the `OpenApiCompiler::compile()` signature does
not change — only the internal construction changes. External callers always
receive `serde_json::Value`.

## Files Affected

| File | Change |
|---|---|
| `crates/fdb-reflection/src/compilers/openapi.rs` | Implement `OpenApiCompiler::compile()` |
| `crates/fdb-reflection/src/compilers/mod.rs` | Export `openapi` module |
| `crates/fdb-gateway/src/main.rs` | Add `GET /openapi.json` route |

## Gate Criteria

Tests in `crates/fdb-reflection/tests/openapi_compiler.rs`:

- `test_openapi_version_is_3_1_0` — top-level `openapi` field is `"3.1.0"`
- `test_every_table_has_crud_paths` — each table produces GET/POST/PATCH/DELETE entries
- `test_column_types_map_correctly` — `integer` columns map to `{ "type": "integer" }`,
  `uuid` maps to `{ "type": "string", "format": "uuid" }`, etc.
- `test_function_has_post_path` — each function produces `POST /rpc/<schema>/<fn>` path
- `test_openapi_doc_is_valid_json` — output deserializes without error
- `test_bearer_security_scheme_present` — `components.securitySchemes.bearerAuth` exists
- HTTP integration: `GET /openapi.json` returns `Content-Type: application/json` and
  `"openapi": "3.1.0"` at root
