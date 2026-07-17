//! Tests for the MCP compiler.

use super::schema::pg_type_to_json_type;
use super::*;
use crate::model::{Column, ForeignKey, Table};

fn make_table(schema: &str, name: &str, columns: Vec<Column>, pk: Vec<&str>) -> Table {
    Table {
        schema: schema.into(),
        name: name.into(),
        columns,
        pk: pk.into_iter().map(String::from).collect(),
        fk: vec![ForeignKey {
            from_col: "user_id".into(),
            to_schema: "public".into(),
            to_table: "users".into(),
            to_col: "id".into(),
        }],
        rls_enabled: true,
        vault_key: None,
    }
}

fn make_model(tables: Vec<Table>) -> DatabaseModel {
    DatabaseModel {
        tables,
        functions: vec![],
        views: vec![],
        version: 1,
    }
}

#[test]
fn compile_generates_5_tools_per_table() {
    let model = make_model(vec![make_table(
        "public",
        "orders",
        vec![
            Column {
                name: "id".into(),
                pg_type: "uuid".into(),
                nullable: false,
                default: Some("gen_random_uuid()".into()),
            },
            Column {
                name: "status".into(),
                pg_type: "text".into(),
                nullable: false,
                default: None,
            },
            Column {
                name: "total".into(),
                pg_type: "numeric".into(),
                nullable: false,
                default: None,
            },
        ],
        vec!["id"],
    )]);
    let result = McpCompiler::compile(&model);
    let tools = result["tools"].as_array().expect("tools array");
    // list, get, create, update, delete = 5
    assert_eq!(tools.len(), 5);
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert!(names.contains(&"list_orders"));
    assert!(names.contains(&"get_orders"));
    assert!(names.contains(&"create_orders"));
    assert!(names.contains(&"update_orders"));
    assert!(names.contains(&"delete_orders"));
}

#[test]
fn compile_excludes_internal_schemas() {
    let model = make_model(vec![
        make_table("public", "orders", vec![], vec![]),
        make_table("flint_meta", "cache_tables", vec![], vec![]),
        make_table("auth", "users", vec![], vec![]),
    ]);
    let result = McpCompiler::compile(&model);
    let tools = result["tools"].as_array().expect("tools array");
    // Only public.orders — no flint_meta or auth tools
    // With no PK on orders: list + get + create = 3 tools
    assert_eq!(tools.len(), 3);
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert!(names
        .iter()
        .all(|n| !n.contains("cache_tables") && !n.contains("users")));
}

#[test]
fn compile_skips_update_delete_for_tables_without_pk() {
    let model = make_model(vec![make_table("public", "logs", vec![], vec![])]);
    let result = McpCompiler::compile(&model);
    let tools = result["tools"].as_array().expect("tools array");
    // No PK → only list + get + create = 3
    assert_eq!(tools.len(), 3);
}

#[test]
fn list_tool_has_pagination_properties() {
    let model = make_model(vec![make_table(
        "public",
        "orders",
        vec![Column {
            name: "id".into(),
            pg_type: "uuid".into(),
            nullable: false,
            default: None,
        }],
        vec!["id"],
    )]);
    let result = McpCompiler::compile(&model);
    let list_tool = &result["tools"][0];
    assert_eq!(list_tool["name"], "list_orders");
    let props = &list_tool["inputSchema"]["properties"];
    assert!(props["limit"]["default"] == 50);
    assert!(props["offset"]["default"] == 0);
    assert!(props["eq"].is_object());
}

#[test]
fn create_tool_skips_auto_generated_columns() {
    let model = make_model(vec![make_table(
        "public",
        "orders",
        vec![
            Column {
                name: "id".into(),
                pg_type: "uuid".into(),
                nullable: false,
                default: Some("gen_random_uuid()".into()),
            },
            Column {
                name: "total".into(),
                pg_type: "numeric".into(),
                nullable: false,
                default: None,
            },
        ],
        vec!["id"],
    )]);
    let result = McpCompiler::compile(&model);
    let create_tool = result["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "create_orders")
        .expect("create tool");
    let props = &create_tool["inputSchema"]["properties"];
    // id should be excluded (gen_random_uuid default), total should be present
    assert!(props.get("total").is_some());
    assert!(props.get("id").is_none());
}

#[test]
fn pg_type_to_json_type_maps_correctly() {
    assert_eq!(pg_type_to_json_type("integer"), "integer");
    assert_eq!(pg_type_to_json_type("bigint"), "integer");
    assert_eq!(pg_type_to_json_type("serial"), "integer");
    assert_eq!(pg_type_to_json_type("numeric"), "number");
    assert_eq!(pg_type_to_json_type("double precision"), "number");
    assert_eq!(pg_type_to_json_type("boolean"), "boolean");
    assert_eq!(pg_type_to_json_type("text"), "string");
    assert_eq!(pg_type_to_json_type("uuid"), "string");
    assert_eq!(pg_type_to_json_type("timestamp with time zone"), "string");
    assert_eq!(pg_type_to_json_type("jsonb"), "object");
    assert_eq!(pg_type_to_json_type("vector(1536)"), "array");
}

#[test]
fn safe_name_strips_public_schema_prefix() {
    assert_eq!(safe_name("public", "orders"), "orders");
    assert_eq!(safe_name("app", "orders"), "app_orders");
}

#[test]
fn compile_filtered_respects_schema_filter() {
    let model = make_model(vec![
        make_table("public", "orders", vec![], vec![]),
        make_table("app", "orders", vec![], vec![]),
    ]);
    let result = McpCompiler::compile_filtered(&model, Some(&["public".into()]));
    let tools = result["tools"].as_array().expect("tools array");
    // Only public.orders — 3 tools (no PK)
    assert_eq!(tools.len(), 3);
    assert!(tools.iter().all(|t| {
        t["name"].as_str().unwrap_or("").contains("orders")
            && !t["name"].as_str().unwrap_or("").starts_with("list_app_")
    }));
}

#[test]
fn compile_handles_empty_model() {
    let model = make_model(vec![]);
    let result = McpCompiler::compile(&model);
    assert_eq!(result["tools"].as_array().unwrap().len(), 0);
}

#[test]
fn compile_generates_function_tools() {
    use crate::model::{ArgMeta, FnMeta};
    let model = DatabaseModel {
        tables: vec![],
        functions: vec![FnMeta {
            schema: "public".into(),
            name: "calculate_total".into(),
            args: vec![
                ArgMeta {
                    name: "order_id".into(),
                    pg_type: "uuid".into(),
                },
                ArgMeta {
                    name: "discount".into(),
                    pg_type: "numeric".into(),
                },
            ],
            return_type: "numeric".into(),
            security_definer: false,
        }],
        views: vec![],
        version: 1,
    };
    let result = McpCompiler::compile(&model);
    let tools = result["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "call_calculate_total");
    let props = &tools[0]["inputSchema"]["properties"];
    assert!(props.get("order_id").is_some());
    assert!(props.get("discount").is_some());
    assert_eq!(
        tools[0]["inputSchema"]["required"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}
