//! Per-table, per-view, and per-function MCP tool generators.

use crate::model::{FnMeta, Table};
use serde_json::{json, Value};

use super::safe_name;
use super::schema::{
    column_list_description, eq_filter_properties, insert_properties, pg_type_to_json_type,
    pk_properties, pk_required, update_properties,
};

// ─── Per-table tool generators ──────────────────────────────────────────────

/// Generate the 5 CRUD tools for a table.
pub(super) fn table_tools(table: &Table) -> Vec<Value> {
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

pub(super) fn view_tool(schema: &str, name: &str) -> Value {
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

pub(super) fn function_tool(func: &FnMeta) -> Option<Value> {
    function_tool_meta(func, None)
}

pub(super) fn function_tool_meta(func: &FnMeta, _schemas: Option<&[String]>) -> Option<Value> {
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
