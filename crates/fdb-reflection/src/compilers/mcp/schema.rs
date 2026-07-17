//! JSON Schema property helpers used to build MCP tool `inputSchema` values.

use crate::model::Table;
use serde_json::{json, Value};

/// Build the `eq` filter properties object from table columns.
pub(super) fn eq_filter_properties(table: &Table) -> Value {
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
pub(super) fn pk_properties(table: &Table) -> Value {
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
pub(super) fn pk_required(table: &Table) -> Value {
    json!(table.pk)
}

/// Build insert properties (all non-auto-generated columns).
pub(super) fn insert_properties(table: &Table) -> Value {
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
pub(super) fn update_properties(table: &Table) -> Value {
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
pub(super) fn column_list_description(table: &Table) -> String {
    let names: Vec<&str> = table.columns.iter().map(|c| c.name.as_str()).collect();
    names.join(", ")
}

/// Map a Postgres type string to a JSON Schema type.
pub(super) fn pg_type_to_json_type(pg_type: &str) -> &'static str {
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
