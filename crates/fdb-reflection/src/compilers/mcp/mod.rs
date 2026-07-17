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

mod schema;
mod tools;

#[cfg(test)]
mod tests;

use crate::model::DatabaseModel;
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
            tools.extend(tools::table_tools(table));
        }

        for view in &model.views {
            if is_excluded(&view.schema) {
                continue;
            }
            tools.push(tools::view_tool(view.schema.as_str(), view.name.as_str()));
        }

        for func in &model.functions {
            if is_excluded(&func.schema) {
                continue;
            }
            if let Some(tool) = tools::function_tool(func) {
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
            tools.extend(tools::table_tools(table));
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
            tools.push(tools::view_tool(&view.schema, &view.name));
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
            if let Some(tool) = tools::function_tool_meta(func, schemas) {
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
