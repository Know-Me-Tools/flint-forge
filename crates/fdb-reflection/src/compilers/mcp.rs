//! MCP Tools Compiler — generates typed MCP tool definitions from `DatabaseModel`.
//!
//! Each table becomes 5 CRUD tools (`list_`, `get_`, `create_`, `update_`, `delete_`).
//! Each function becomes a `call_` tool. Each view becomes a read-only `list_` tool.
//!
//! Internal schemas (`flint_meta`, `flint_a2ui`, `auth`, etc.) are excluded by
//! default so the tool manifest stays focused on user-visible data.
//!
//! The output is a JSON value shaped as an MCP `tools/list` result:
//! ```json
//! { "tools": [ { "name": "list_orders", "description": "...", "inputSchema": {...} } ] }
//! ```

use crate::model::{DatabaseModel, Table};
use serde_json::{json, Value};

/// Schemas excluded from MCP tool generation by default (internal infrastructure).
const EXCLUDED_SCHEMAS: &[&str] = &[
    "flint_meta",
    "flint_a2ui",
    "auth",
    "graphql_public",
    "_flint",
    "pg_catalog",
    "information_schema",
    "pg_toast",
];

/// Compiles a `DatabaseModel` into MCP tool descriptors.
///
/// Returns a JSON value with a `tools` array ready to be served at
/// `/mcp/v1/tools` or embedded in an AG-UI `StateSnapshot`.
pub struct McpCompiler;

impl McpCompiler {
    /// Compile all MCP tools from the database model.
    pub fn compile(model: &DatabaseModel) -> Value {
        let mut tools: Vec<Value> = Vec::new();

        for table in &model.tables {
            if is_excluded(&table.schema) {
                continue;
            }
            tools.extend(table_tools(table));
        }

        for view in &model.views {
            if is_excluded(&view.schema) {
                continue;
            }
            tools.push(view_tool(view.schema.as_str(), view.name.as_str()));
        }

        for func in &model.functions {
            if is_excluded(&func.schema) {
                continue;
            }
            if let Some(tool) = function_tool(func) {
                tools.push(tool);
            }
        }

        json!({ "tools": tools })
    }

    /// Compile tools filtered to a specific set of schemas.
    /// Pass `None` to include all non-excluded schemas.
    pub fn compile_filtered(model: &DatabaseModel, schemas: Option<&[String]>) -> Value {
        let mut tools: Vec<Value> = Vec::new();

        for table in &model.tables {
            if is_excluded(&table.schema) {
                continue;
            }
            if let Some(allowed) = schemas {
                if !allowed.contains(&table.schema) {
                    continue;
                }
            }
            tools.extend(table_tools(table));
        }

        for view in &model.views {
            if is_excluded(&view.schema) {
                continue;
            }
            if let Some(allowed) = schemas {
                if !allowed.contains(&view.schema) {
                    continue;
                }
            }
            tools.push(view_tool(&view.schema, &view.name));
        }

        for func in &model.functions {
            if is_excluded(&func.schema) {
                continue;
            }
            if let Some(allowed) = schemas {
                if !allowed.contains(&func.schema) {
                    continue;
                }
            }
            if let Some(tool) = function_tool_meta(func, schemas) {
                tools.push(tool);
            }
        }

        json!({ "tools": tools })
    }
}

/// Check if a schema is in the exclusion list.
fn is_excluded(schema: &str) -> bool {
    EXCLUDED_SCHEMAS.contains(&schema)
}

/// Sanitize a table/function name into a valid MCP tool name (snake_case, alphanumeric + underscore).
fn safe_name(schema: &str, name: &str) -> String {
    let combined = if schema == "public" {
        name.to_owned()
    } else {
        format!("{schema}_{name}")
    };
    combined
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

// ─── Per-table tool generators ──────────────────────────────────────────────

/// Generate the 5 CRUD tools for a table.
fn table_tools(table: &Table) -> Vec<Value> {
    let tn = safe_name(&table.schema, &table.name);
    let full_name = format!("{}.{}", table.schema, table.name);
    let has_pk = !table.pk.is_empty();

    let mut tools = vec![
        list_tool(&tn, &full_name, table),
        get_tool(&tn, &full_name, table, has_pk),
        create_tool(&tn, &full_name, table),
    ];

    if has_pk {
        tools.push(update_tool(&tn, &full_name, table));
        tools.push(delete_tool(&tn, &full_name, table));
    }

    tools
}

fn list_tool(table_name: &str, full_name: &str, table: &Table) -> Value {
    let select_desc = column_list_description(table);
    json!({
        "name": format!("list_{table_name}"),
        "description": format!("List rows from {full_name}. Supports column selection, equality filtering, ordering, and pagination. {select_desc}"),
        "inputSchema": {
            "type": "object",
            "properties": {
                "select": {
                    "type": "string",
                    "description": format!("Comma-separated column names. Available: {select_desc}. Default: *")
                },
                "eq": {
                    "type": "object",
                    "description": "Equality filters as key-value pairs, e.g. {\"status\": \"shipped\"}",
                    "properties": eq_filter_properties(table)
                },
                "order": {
                    "type": "string",
                    "description": "Order expression: \"column.dir\" where dir is asc or desc. Default: primary key ascending."
                },
                "limit": { "type": "integer", "minimum": 1, "maximum": 1000, "default": 50 },
                "offset": { "type": "integer", "minimum": 0, "default": 0 }
            }
        }
    })
}

fn get_tool(table_name: &str, full_name: &str, table: &Table, has_pk: bool) -> Value {
    let pk_desc = if has_pk {
        table.pk.join(", ")
    } else {
        "row identifier".to_owned()
    };
    json!({
        "name": format!("get_{table_name}"),
        "description": format!("Get a single row from {full_name} by primary key ({pk_desc})."),
        "inputSchema": {
            "type": "object",
            "required": pk_required(table),
            "properties": pk_properties(table)
        }
    })
}

fn create_tool(table_name: &str, full_name: &str, table: &Table) -> Value {
    json!({
        "name": format!("create_{table_name}"),
        "description": format!("Insert a new row into {full_name}."),
        "inputSchema": {
            "type": "object",
            "properties": insert_properties(table)
        }
    })
}

fn update_tool(table_name: &str, full_name: &str, table: &Table) -> Value {
    json!({
        "name": format!("update_{table_name}"),
        "description": format!("Update a row in {full_name} by primary key."),
        "inputSchema": {
            "type": "object",
            "required": pk_required(table),
            "properties": update_properties(table)
        }
    })
}

fn delete_tool(table_name: &str, full_name: &str, table: &Table) -> Value {
    json!({
        "name": format!("delete_{table_name}"),
        "description": format!("Delete a row from {full_name} by primary key."),
        "inputSchema": {
            "type": "object",
            "required": pk_required(table),
            "properties": pk_properties(table)
        }
    })
}

// ─── View tool ──────────────────────────────────────────────────────────────

fn view_tool(schema: &str, name: &str) -> Value {
    let vn = safe_name(schema, name);
    let full_name = format!("{schema}.{name}");
    json!({
        "name": format!("list_{vn}"),
        "description": format!("List rows from view {full_name}. Read-only."),
        "inputSchema": {
            "type": "object",
            "properties": {
                "limit": { "type": "integer", "minimum": 1, "maximum": 1000, "default": 50 },
                "offset": { "type": "integer", "minimum": 0, "default": 0 }
            }
        }
    })
}

// ─── Function tool ──────────────────────────────────────────────────────────

fn function_tool(func: &crate::model::FnMeta) -> Option<Value> {
    function_tool_meta(func, None)
}

fn function_tool_meta(func: &crate::model::FnMeta, _schemas: Option<&[String]>) -> Option<Value> {
    // Skip functions with no name or internal signatures
    if func.name.is_empty() {
        return None;
    }
    let fn_name = safe_name(&func.schema, &func.name);
    let full_name = format!("{}.{}", func.schema, func.name);

    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();
    for arg in &func.args {
        let json_type = pg_type_to_json_type(&arg.pg_type);
        properties.insert(
            arg.name.clone(),
            json!({
                "type": json_type,
                "description": format!("Argument {} of type {}", arg.name, arg.pg_type)
            }),
        );
        // Require all args by default; functions with defaults would need reflection metadata
        required.push(arg.name.clone());
    }

    let mut schema = json!({
        "type": "object",
        "properties": properties,
    });
    if !required.is_empty() {
        schema["required"] = json!(required);
    }

    Some(json!({
        "name": format!("call_{fn_name}"),
        "description": format!("Call Postgres function {full_name}(). Returns {return_type}.", return_type = func.return_type),
        "inputSchema": schema
    }))
}

// ─── JSON Schema property helpers ───────────────────────────────────────────

/// Build the `eq` filter properties object from table columns.
fn eq_filter_properties(table: &Table) -> Value {
    let mut props = serde_json::Map::new();
    for col in &table.columns {
        props.insert(
            col.name.clone(),
            json!({
                "description": format!("Filter where {} equals this value", col.name)
            }),
        );
    }
    Value::Object(props)
}

/// Build the PK properties object.
fn pk_properties(table: &Table) -> Value {
    let mut props = serde_json::Map::new();
    for pk_col in &table.pk {
        if let Some(col) = table.columns.iter().find(|c| &c.name == pk_col) {
            props.insert(
                col.name.clone(),
                json!({
                    "type": pg_type_to_json_type(&col.pg_type),
                    "description": format!("Primary key column: {} ({})", col.name, col.pg_type)
                }),
            );
        }
    }
    Value::Object(props)
}

/// Build the PK required array.
fn pk_required(table: &Table) -> Value {
    json!(table.pk)
}

/// Build insert properties (all non-auto-generated columns).
fn insert_properties(table: &Table) -> Value {
    let mut props = serde_json::Map::new();
    for col in &table.columns {
        // Skip serial/identity columns (they have defaults that auto-generate)
        let is_auto = col.default.as_deref().is_some_and(|d| {
            d.contains("nextval") || d.contains("gen_random_uuid") || d.contains("identity")
        });
        if is_auto {
            continue;
        }
        let mut field = json!({
            "type": pg_type_to_json_type(&col.pg_type),
            "description": format!("Column {} ({})", col.name, col.pg_type)
        });
        if col.nullable {
            field["nullable"] = json!(true);
        }
        props.insert(col.name.clone(), field);
    }
    Value::Object(props)
}

/// Build update properties (PK + all updatable columns).
fn update_properties(table: &Table) -> Value {
    let mut props = serde_json::Map::new();
    // Include PK columns for the WHERE clause
    for pk_col in &table.pk {
        if let Some(col) = table.columns.iter().find(|c| &c.name == pk_col) {
            props.insert(
                col.name.clone(),
                json!({
                    "type": pg_type_to_json_type(&col.pg_type),
                    "description": format!("Primary key: {} ({})", col.name, col.pg_type)
                }),
            );
        }
    }
    // Include all columns as optional update fields
    for col in &table.columns {
        if table.pk.contains(&col.name) {
            continue;
        }
        props.insert(
            col.name.clone(),
            json!({
                "type": pg_type_to_json_type(&col.pg_type),
                "description": format!("Update column {} ({})", col.name, col.pg_type)
            }),
        );
    }
    Value::Object(props)
}

/// Comma-separated column names for descriptions.
fn column_list_description(table: &Table) -> String {
    let names: Vec<&str> = table.columns.iter().map(|c| c.name.as_str()).collect();
    names.join(", ")
}

/// Map a Postgres type string to a JSON Schema type.
fn pg_type_to_json_type(pg_type: &str) -> &'static str {
    let lower = pg_type.to_ascii_lowercase();
    if lower.starts_with("int")
        || lower.starts_with("serial")
        || lower == "bigint"
        || lower == "bigserial"
        || lower == "smallint"
        || lower == "integer"
    {
        "integer"
    } else if lower.starts_with("float")
        || lower.starts_with("numeric")
        || lower.starts_with("decimal")
        || lower == "real"
        || lower == "double precision"
        || lower == "money"
    {
        "number"
    } else if lower == "boolean" || lower == "bool" {
        "boolean"
    } else if lower.starts_with("json") {
        "object"
    } else if lower.starts_with("vector") {
        "array"
    } else {
        // text, varchar, uuid, timestamp, date, time, etc. all map to string
        "string"
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Column, ForeignKey};

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
}
