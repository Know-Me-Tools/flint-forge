use serde_json::{Value, json};

use crate::model::{ArgMeta, Column, DatabaseModel, FnMeta, Table};

/// Compiles a `DatabaseModel` into an OpenAPI 3.1.0 JSON document.
///
/// The document is served at `GET /openapi.json` by `fdb-gateway` and is used
/// for SDK generation, MCP tool descriptions (Phase 7), and client discovery.
/// No new crate dependencies — hand-rolled `serde_json::Value` construction.
pub struct OpenApiCompiler;

impl OpenApiCompiler {
    pub fn compile(model: &DatabaseModel) -> Value {
        let mut paths = serde_json::Map::new();
        let mut schemas = serde_json::Map::new();

        for table in &model.tables {
            let schema_name = format!("{}_{}", table.schema, table.name);
            schemas.insert(schema_name.clone(), table_to_schema(table));

            let collection_path = format!("/{}/{}", table.schema, table.name);
            let item_path = format!("/{}/{}/{{id}}", table.schema, table.name);

            paths.insert(collection_path, table_collection_paths(table, &schema_name));
            paths.insert(item_path, table_item_paths(table, &schema_name));
        }

        for func in &model.functions {
            let rpc_path = format!("/rpc/{}/{}", func.schema, func.name);
            paths.insert(rpc_path, fn_path(func));
        }

        json!({
            "openapi": "3.1.0",
            "info": {
                "title": "Flint Quarry REST API",
                "version": model.version.to_string(),
                "description": "Auto-generated from live database schema via flint_meta"
            },
            "paths": paths,
            "components": {
                "schemas": schemas,
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

/// Map a Postgres type string to a JSON Schema type object.
fn pg_type_to_json_schema(pg_type: &str) -> Value {
    let t = pg_type.trim().to_lowercase();
    if matches!(
        t.as_str(),
        "integer" | "int" | "int4" | "int8" | "bigint" | "smallint" | "int2" | "serial" | "bigserial"
    ) {
        return json!({"type": "integer"});
    }
    if matches!(
        t.as_str(),
        "numeric" | "float4" | "float8" | "real" | "double precision" | "decimal"
    ) {
        return json!({"type": "number"});
    }
    if matches!(t.as_str(), "boolean" | "bool") {
        return json!({"type": "boolean"});
    }
    if t == "uuid" {
        return json!({"type": "string", "format": "uuid"});
    }
    if matches!(t.as_str(), "timestamp" | "timestamptz" | "timestamp with time zone" | "timestamp without time zone") {
        return json!({"type": "string", "format": "date-time"});
    }
    if t == "date" {
        return json!({"type": "string", "format": "date"});
    }
    if matches!(t.as_str(), "jsonb" | "json") {
        return json!({});
    }
    if t.starts_with("vector") {
        return json!({"type": "array", "items": {"type": "number"}, "description": "pgvector embedding"});
    }
    if matches!(t.as_str(), "text" | "varchar" | "character varying" | "char" | "name" | "citext") {
        return json!({"type": "string"});
    }
    json!({"type": "string", "description": format!("Postgres type: {pg_type}")})
}

/// Build a JSON Schema object for a table's columns.
fn table_to_schema(table: &Table) -> Value {
    let mut properties = serde_json::Map::new();
    let mut required: Vec<Value> = Vec::new();

    for col in &table.columns {
        let schema = pg_type_to_json_schema(&col.pg_type);
        properties.insert(col.name.clone(), schema);
        if !col.nullable && col.default.is_none() && !table.pk.contains(&col.name) {
            required.push(json!(col.name));
        }
    }

    let mut obj = json!({
        "type": "object",
        "properties": properties,
        "x-flint-rls-enabled": table.rls_enabled
    });
    if !required.is_empty() {
        obj["required"] = json!(required);
    }
    obj
}

/// Build filter query parameters for a GET list endpoint (one per column per operator).
fn filter_params(columns: &[Column]) -> Vec<Value> {
    const OPERATORS: &[&str] = &[
        "eq", "neq", "gt", "gte", "lt", "lte",
        "like", "ilike", "is", "in", "cs", "cd",
    ];

    let mut params: Vec<Value> = Vec::new();

    for col in columns {
        for op in OPERATORS {
            params.push(json!({
                "name": format!("{}.{}", col.name, op),
                "in": "query",
                "required": false,
                "schema": {"type": "string"},
                "description": format!("Filter: {} {} value", col.name, op)
            }));
        }
    }

    params.push(json!({
        "name": "order",
        "in": "query",
        "required": false,
        "schema": {"type": "string"},
        "description": "Order: <column>.(asc|desc)"
    }));

    params.push(json!({
        "name": "Range",
        "in": "header",
        "required": false,
        "schema": {"type": "string", "pattern": r"items=\d+-\d+"},
        "description": "Pagination range header"
    }));

    params
}

/// Build GET + POST paths for the collection endpoint `/<schema>/<table>`.
fn table_collection_paths(table: &Table, schema_ref: &str) -> Value {
    let ref_path = format!("#/components/schemas/{schema_ref}");
    let params = filter_params(&table.columns);

    json!({
        "get": {
            "summary": format!("List {}.{}", table.schema, table.name),
            "parameters": params,
            "security": [{"bearerAuth": []}],
            "responses": {
                "200": {
                    "description": "Row list",
                    "content": {"application/json": {"schema": {"type": "array", "items": {"$ref": ref_path}}}}
                }
            }
        },
        "post": {
            "summary": format!("Insert into {}.{}", table.schema, table.name),
            "security": [{"bearerAuth": []}],
            "requestBody": {
                "required": true,
                "content": {"application/json": {"schema": {"$ref": ref_path}}}
            },
            "responses": {
                "201": {
                    "description": "Created row",
                    "content": {"application/json": {"schema": {"$ref": ref_path}}}
                }
            }
        }
    })
}

/// Build PATCH + DELETE paths for the item endpoint `/<schema>/<table>/{id}`.
fn table_item_paths(table: &Table, schema_ref: &str) -> Value {
    let ref_path = format!("#/components/schemas/{schema_ref}");
    let id_param = json!({
        "name": "id",
        "in": "path",
        "required": true,
        "schema": {"type": "string"},
        "description": "Row primary key"
    });

    json!({
        "patch": {
            "summary": format!("Update {}.{}", table.schema, table.name),
            "parameters": [id_param.clone()],
            "security": [{"bearerAuth": []}],
            "requestBody": {
                "required": true,
                "content": {"application/json": {"schema": {"$ref": ref_path}}}
            },
            "responses": {
                "200": {
                    "description": "Updated row",
                    "content": {"application/json": {"schema": {"$ref": ref_path}}}
                }
            }
        },
        "delete": {
            "summary": format!("Delete {}.{}", table.schema, table.name),
            "parameters": [id_param],
            "security": [{"bearerAuth": []}],
            "responses": {
                "200": {
                    "description": "Deleted row",
                    "content": {"application/json": {"schema": {"$ref": ref_path}}}
                }
            }
        }
    })
}

/// Build the request body schema for an RPC function's arguments.
fn fn_args_schema(args: &[ArgMeta]) -> Value {
    let mut properties = serde_json::Map::new();
    for arg in args {
        properties.insert(arg.name.clone(), pg_type_to_json_schema(&arg.pg_type));
    }
    json!({"type": "object", "properties": properties})
}

/// Build a `POST /rpc/<schema>/<fn>` path entry.
fn fn_path(func: &FnMeta) -> Value {
    json!({
        "post": {
            "summary": format!("Call {}.{}", func.schema, func.name),
            "description": format!("Postgres function: {}.{}", func.schema, func.name),
            "security": [{"bearerAuth": []}],
            "requestBody": {
                "required": true,
                "content": {"application/json": {"schema": fn_args_schema(&func.args)}}
            },
            "responses": {
                "200": {
                    "description": "Function result",
                    "content": {"application/json": {"schema": {"type": "array"}}}
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ArgMeta, Column, DatabaseModel, FnMeta, Table};

    fn minimal_model() -> DatabaseModel {
        DatabaseModel {
            tables: vec![Table {
                schema: "public".into(),
                name: "items".into(),
                columns: vec![
                    Column { name: "id".into(), pg_type: "uuid".into(), nullable: false, default: Some("gen_random_uuid()".into()) },
                    Column { name: "name".into(), pg_type: "text".into(), nullable: false, default: None },
                    Column { name: "score".into(), pg_type: "integer".into(), nullable: true, default: None },
                ],
                pk: vec!["id".into()],
                fk: vec![],
                rls_enabled: true,
                vault_key: None,
            }],
            functions: vec![FnMeta {
                schema: "public".into(),
                name: "find_similar".into(),
                args: vec![
                    ArgMeta { name: "query_vec".into(), pg_type: "vector(3)".into() },
                    ArgMeta { name: "max_results".into(), pg_type: "integer".into() },
                ],
                return_type: "SETOF items".into(),
                security_definer: false,
            }],
            views: vec![],
            version: 42,
        }
    }

    #[test]
    fn test_openapi_version_is_3_1_0() {
        let doc = OpenApiCompiler::compile(&minimal_model());
        assert_eq!(doc["openapi"], "3.1.0");
    }

    #[test]
    fn test_info_version_matches_model_version() {
        let doc = OpenApiCompiler::compile(&minimal_model());
        assert_eq!(doc["info"]["version"], "42");
    }

    #[test]
    fn test_every_table_has_collection_and_item_paths() {
        let doc = OpenApiCompiler::compile(&minimal_model());
        let paths = doc["paths"].as_object().unwrap();
        assert!(paths.contains_key("/public/items"), "collection path missing");
        assert!(paths.contains_key("/public/items/{id}"), "item path missing");
    }

    #[test]
    fn test_collection_path_has_get_and_post() {
        let doc = OpenApiCompiler::compile(&minimal_model());
        let entry = &doc["paths"]["/public/items"];
        assert!(entry["get"].is_object(), "GET missing");
        assert!(entry["post"].is_object(), "POST missing");
    }

    #[test]
    fn test_item_path_has_patch_and_delete() {
        let doc = OpenApiCompiler::compile(&minimal_model());
        let entry = &doc["paths"]["/public/items/{id}"];
        assert!(entry["patch"].is_object(), "PATCH missing");
        assert!(entry["delete"].is_object(), "DELETE missing");
    }

    #[test]
    fn test_function_has_post_path() {
        let doc = OpenApiCompiler::compile(&minimal_model());
        let paths = doc["paths"].as_object().unwrap();
        assert!(paths.contains_key("/rpc/public/find_similar"), "rpc path missing");
        let entry = &doc["paths"]["/rpc/public/find_similar"];
        assert!(entry["post"].is_object());
    }

    #[test]
    fn test_column_types_map_correctly() {
        assert_eq!(pg_type_to_json_schema("uuid")["format"], "uuid");
        assert_eq!(pg_type_to_json_schema("integer")["type"], "integer");
        assert_eq!(pg_type_to_json_schema("boolean")["type"], "boolean");
        assert_eq!(pg_type_to_json_schema("text")["type"], "string");
        assert_eq!(pg_type_to_json_schema("jsonb"), json!({}));
        assert_eq!(pg_type_to_json_schema("vector(384)")["type"], "array");
        assert_eq!(pg_type_to_json_schema("timestamptz")["format"], "date-time");
    }

    #[test]
    fn test_bearer_security_scheme_present() {
        let doc = OpenApiCompiler::compile(&minimal_model());
        let scheme = &doc["components"]["securitySchemes"]["bearerAuth"];
        assert_eq!(scheme["type"], "http");
        assert_eq!(scheme["scheme"], "bearer");
    }

    #[test]
    fn test_schema_component_created_for_table() {
        let doc = OpenApiCompiler::compile(&minimal_model());
        let schemas = doc["components"]["schemas"].as_object().unwrap();
        assert!(schemas.contains_key("public_items"), "schema component missing");
        let schema = &doc["components"]["schemas"]["public_items"];
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["id"].is_object());
        assert!(schema["properties"]["name"].is_object());
    }

    #[test]
    fn test_rls_extension_field_on_schema() {
        let doc = OpenApiCompiler::compile(&minimal_model());
        assert_eq!(doc["components"]["schemas"]["public_items"]["x-flint-rls-enabled"], true);
    }

    #[test]
    fn test_filter_params_include_operators_for_columns() {
        let doc = OpenApiCompiler::compile(&minimal_model());
        let params = doc["paths"]["/public/items"]["get"]["parameters"].as_array().unwrap();
        let param_names: Vec<&str> = params.iter()
            .filter_map(|p| p["name"].as_str())
            .collect();
        assert!(param_names.contains(&"name.eq"), "name.eq filter param missing");
        assert!(param_names.contains(&"score.gt"), "score.gt filter param missing");
        assert!(param_names.contains(&"order"), "order param missing");
    }

    #[test]
    fn test_vector_arg_in_function_schema() {
        let doc = OpenApiCompiler::compile(&minimal_model());
        let body_schema = &doc["paths"]["/rpc/public/find_similar"]["post"]["requestBody"]["content"]["application/json"]["schema"];
        let props = body_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("query_vec"), "query_vec missing");
        assert_eq!(props["query_vec"]["type"], "array");
    }

    #[test]
    fn test_doc_is_valid_json_value() {
        let doc = OpenApiCompiler::compile(&minimal_model());
        let serialized = serde_json::to_string(&doc).unwrap();
        let _: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    }
}
