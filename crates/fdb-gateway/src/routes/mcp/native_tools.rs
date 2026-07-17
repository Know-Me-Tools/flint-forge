//! Native MCP tools: `a2ui_generate_form`, `a2ui_generate_grid`, and
//! `a2ui_resolve_tokens`. Unlike the A2UI-delegated tools in
//! `super::dispatch`, these query `flint_a2ui` bindings directly.

use forge_identity::RlsContext;
use serde_json::{json, Value};
use sqlx::types::Json as SqlxJson;
use sqlx::FromRow;
use uuid::Uuid;

use super::protocol::{RpcError, INTERNAL_ERROR, INVALID_PARAMS};
use super::McpState;

/// Binding row for native tool queries.
#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct BindingForToolRow {
    slug: String,
    primitive_type: String,
    binding_type: String,
    table_schema: String,
    table_name: String,
    config: SqlxJson<Value>,
}

/// `a2ui_generate_form` — find the form binding for a table and return a
/// component instance ready for rendering.
pub(super) async fn generate_form(
    state: &McpState,
    _who: &RlsContext,
    args: &Value,
) -> Result<Value, RpcError> {
    let schema = args
        .get("schema")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError::new(INVALID_PARAMS, "schema required"))?;
    let table = args
        .get("table")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError::new(INVALID_PARAMS, "table required"))?;
    let row = find_binding(state, schema, table, "form").await?;
    Ok(json!({
        "component": "form",
        "table": { "schema": schema, "name": table },
        "binding": {
            "slug": row.slug,
            "primitive_type": row.primitive_type,
            "binding_type": row.binding_type,
            "config": row.config.0,
        },
        "fields": row.config.0.get("fields").cloned().unwrap_or(Value::Array(vec![])),
    }))
}

/// `a2ui_generate_grid` — find the grid/data-table binding for a table.
pub(super) async fn generate_grid(
    state: &McpState,
    _who: &RlsContext,
    args: &Value,
) -> Result<Value, RpcError> {
    let schema = args
        .get("schema")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError::new(INVALID_PARAMS, "schema required"))?;
    let table = args
        .get("table")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError::new(INVALID_PARAMS, "table required"))?;
    let row = find_binding(state, schema, table, "grid").await?;
    Ok(json!({
        "component": "data-grid",
        "table": { "schema": schema, "name": table },
        "binding": {
            "slug": row.slug,
            "primitive_type": row.primitive_type,
            "binding_type": row.binding_type,
            "config": row.config.0,
        },
        "columns": row.config.0.get("columns").cloned().unwrap_or(Value::Array(vec![])),
    }))
}

/// Locate the binding row for a (schema, table, binding_type). Falls back to
/// any binding for the table when an exact type match is missing.
async fn find_binding(
    state: &McpState,
    schema: &str,
    table: &str,
    binding_type: &str,
) -> Result<BindingForToolRow, RpcError> {
    let row: Option<BindingForToolRow> = sqlx::query_as(
        "SELECT c.slug, c.primitive_type, b.binding_type, b.table_schema, b.table_name, b.config
         FROM flint_a2ui.bindings b
         JOIN flint_a2ui.components c ON c.id = b.component_id
         WHERE b.table_schema = $1 AND b.table_name = $2 AND b.binding_type = $3
         LIMIT 1",
    )
    .bind(schema)
    .bind(table)
    .bind(binding_type)
    .fetch_optional(&state.a2ui.pool)
    .await
    .map_err(|e| RpcError::new(INTERNAL_ERROR, e.to_string()))?;

    if let Some(r) = row {
        return Ok(r);
    }
    // Fallback: any binding for the table.
    let row: Option<BindingForToolRow> = sqlx::query_as(
        "SELECT c.slug, c.primitive_type, b.binding_type, b.table_schema, b.table_name, b.config
         FROM flint_a2ui.bindings b
         JOIN flint_a2ui.components c ON c.id = b.component_id
         WHERE b.table_schema = $1 AND b.table_name = $2
         LIMIT 1",
    )
    .bind(schema)
    .bind(table)
    .fetch_optional(&state.a2ui.pool)
    .await
    .map_err(|e| RpcError::new(INTERNAL_ERROR, e.to_string()))?;
    row.ok_or_else(|| RpcError::new(INVALID_PARAMS, format!("no binding for {schema}.{table}")))
}

/// `a2ui_resolve_tokens` — return the design-token palette for an application
/// slug and optional component category. Today the base palette is returned;
/// application-specific token overrides are a follow-on.
pub(super) async fn resolve_tokens(
    state: &McpState,
    _who: &RlsContext,
    args: &Value,
) -> Result<Value, RpcError> {
    let app_slug = args
        .get("application_slug")
        .and_then(Value::as_str)
        .unwrap_or("flint-base");
    let category = args.get("category").and_then(Value::as_str);

    // Confirm the application exists; reserved for future app-scoped tokens.
    let app_id: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM flint_a2ui.applications WHERE slug = $1")
            .bind(app_slug)
            .fetch_optional(&state.a2ui.pool)
            .await
            .map_err(|e| RpcError::new(INTERNAL_ERROR, e.to_string()))?;
    // The existence check must actually gate the response: previously the
    // fetched row was discarded (`let _ = app_id;`), so an unknown
    // `application_slug` silently fell through to the base palette instead
    // of surfacing an error.
    app_id.ok_or_else(|| {
        RpcError::new(INVALID_PARAMS, format!("unknown application '{app_slug}'"))
    })?;

    Ok(json!({
        "application": app_slug,
        "category": category,
        "tokens": {
            "color": {
                "primary": "#2563eb",
                "surface": "#ffffff",
                "text":    "#0f172a",
            },
            "spacing": { "unit": 4 },
            "radius":  { "md": "6px" },
            "typography": {
                "font_family": "Inter, system-ui, sans-serif",
                "size": { "md": "14px" },
            },
        }
    }))
}
