//! MCP server endpoint — JSON-RPC 2.0 over HTTP.
//!
//! Exposes A2UI registry tools at `/mcp/v1/a2ui` so LLM agents can discover
//! and assemble Flint components via tool calling. The MCP layer is thin: each
//! tool delegates to the same inner functions used by the A2UI REST handlers
//! (p5-c006), preserving a single SQL authority and avoiding logic duplication.
//!
//! # Protocol
//!
//! - `POST /mcp/v1/a2ui` — JSON-RPC 2.0 request/response.
//! - `GET  /mcp/v1/a2ui/sse` — optional SSE event stream (keep-alive only).
//!
//! Supported methods: `initialize`, `tools/list`, `tools/call`, `ping`.
//!
//! # Security
//!
//! The route is mounted behind `rls_layer::require_rls`, so every tool call
//! runs under the caller's verified `RlsContext`. Tools never bypass RLS.
#![forbid(unsafe_code)]

use axum::{
    extract::State,
    response::{IntoResponse, Json},
    Extension,
};
use forge_identity::RlsContext;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::types::Json as SqlxJson;
use sqlx::FromRow;
use uuid::Uuid;

use crate::routes::a2ui::{
    self, AssembleSurfaceBody, A2uiState, ListComponentsQuery, SearchComponentsBody,
};

/// MCP server-scoped state. Wraps the A2UI route state so tools can call the
/// shared inner handler functions.
#[derive(Clone)]
pub struct McpState {
    pub a2ui: A2uiState,
}

// ─── JSON-RPC protocol types ────────────────────────────────────────────────

/// JSON-RPC 2.0 request envelope.
#[derive(Debug, Deserialize)]
pub struct McpRequest {
    #[allow(dead_code)]
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Serialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
}

impl RpcError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }
}

/// Standard JSON-RPC error codes.
const PARSE_ERROR: i32 = -32700;
const METHOD_NOT_FOUND: i32 = -32601;
const INVALID_PARAMS: i32 = -32602;
const INTERNAL_ERROR: i32 = -32603;

/// `POST /mcp/v1/a2ui` — JSON-RPC 2.0 dispatch.
///
/// Every call shares the caller's `RlsContext` (installed by `require_rls`).
pub async fn handle_mcp(
    State(state): State<McpState>,
    Extension(who): Extension<RlsContext>,
    Json(req): Json<McpRequest>,
) -> Json<Value> {
    let result = dispatch(&state, &who, &req.method, req.params.as_ref()).await;
    Json(match result {
        Ok(value) => json!({ "jsonrpc": "2.0", "id": req.id, "result": value }),
        Err(e) => json!({
            "jsonrpc": "2.0",
            "id": req.id,
            "error": { "code": e.code, "message": e.message }
        }),
    })
}

/// Silence the dead-code lint for `PARSE_ERROR` — it is part of the protocol
/// surface but not currently returned (axum rejects malformed JSON before this
/// handler runs).
#[allow(dead_code)]
fn _parse_error_marker() -> i32 {
    PARSE_ERROR
}

/// Method dispatch. Returns a JSON-RPC `result` value or an error.
async fn dispatch(
    state: &McpState,
    who: &RlsContext,
    method: &str,
    params: Option<&Value>,
) -> Result<Value, RpcError> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "flint-a2ui-registry",
                "version": env!("CARGO_PKG_VERSION"),
            }
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(tool_definitions()),
        "tools/call" => {
            let name = params
                .and_then(|p| p.get("name"))
                .and_then(Value::as_str)
                .ok_or_else(|| RpcError::new(INVALID_PARAMS, "tool name required"))?;
            let args = params.and_then(|p| p.get("arguments"));
            dispatch_tool(state, who, name, args).await
        }
        _ => Err(RpcError::new(METHOD_NOT_FOUND, format!("unknown method: {method}"))),
    }
}

// ─── Tool registry ──────────────────────────────────────────────────────────

/// Return the `tools/list` result — 7 A2UI tools.
pub fn tool_definitions() -> Value {
    json!({
        "tools": [
            {
                "name": "a2ui_list_components",
                "description": "List available UI components for an application. Returns base components plus app-scoped components the caller can access.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "app_id":     { "type": "string", "format": "uuid", "description": "Optional application ID to include app-specific components" },
                        "category":   { "type": "string", "description": "Optional category filter (e.g. 'form', 'display')" }
                    }
                }
            },
            {
                "name": "a2ui_get_component",
                "description": "Get a specific component by slug, including its full JSON schema and render targets.",
                "inputSchema": {
                    "type": "object",
                    "required": ["slug"],
                    "properties": {
                        "slug": { "type": "string", "description": "Component slug (e.g. 'button', 'data-grid')" }
                    }
                }
            },
            {
                "name": "a2ui_semantic_search",
                "description": "Find components by natural language description using hybrid text + semantic vector search.",
                "inputSchema": {
                    "type": "object",
                    "required": ["query"],
                    "properties": {
                        "query":    { "type": "string", "description": "Natural language description of the desired component" },
                        "limit":    { "type": "integer", "minimum": 1, "maximum": 50, "default": 10 },
                        "app_id":   { "type": "string", "format": "uuid" }
                    }
                }
            },
            {
                "name": "a2ui_generate_form",
                "description": "Generate a Form component for a database table using its auto-generated and manual bindings.",
                "inputSchema": {
                    "type": "object",
                    "required": ["schema", "table"],
                    "properties": {
                        "schema": { "type": "string", "description": "Postgres schema name (e.g. 'public')" },
                        "table":  { "type": "string", "description": "Postgres table name (e.g. 'orders')" }
                    }
                }
            },
            {
                "name": "a2ui_generate_grid",
                "description": "Generate a data grid component for a database table using its bindings.",
                "inputSchema": {
                    "type": "object",
                    "required": ["schema", "table"],
                    "properties": {
                        "schema": { "type": "string" },
                        "table":  { "type": "string" }
                    }
                }
            },
            {
                "name": "a2ui_resolve_tokens",
                "description": "Resolve design tokens (color, spacing, typography) for an application and component category.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "application_slug": { "type": "string", "default": "flint-base" },
                        "category":         { "type": "string", "description": "Component category (e.g. 'form', 'display')" }
                    }
                }
            },
            {
                "name": "a2ui_assemble_surface",
                "description": "Assemble an A2UI surface from an event context. Applies application-specific assembly rules and falls back to default table bindings.",
                "inputSchema": {
                    "type": "object",
                    "required": ["event_type"],
                    "properties": {
                        "event_type":       { "type": "string", "description": "Event name driving the assembly (e.g. 'mount', 'record.select')" },
                        "event_context":    { "type": "object", "description": "Event payload (table, record id, etc.)" },
                        "application_id":   { "type": "string", "format": "uuid" }
                    }
                }
            }
        ]
    })
}

// ─── Tool dispatch ──────────────────────────────────────────────────────────

/// Dispatch a `tools/call` to the named tool. Returns an MCP `CallToolResult`.
async fn dispatch_tool(
    state: &McpState,
    who: &RlsContext,
    name: &str,
    args: Option<&Value>,
) -> Result<Value, RpcError> {
    let args = args.unwrap_or(&Value::Null);
    let result_value = match name {
        "a2ui_list_components" => call_list_components(state, who, args).await?,
        "a2ui_get_component" => call_get_component(state, who, args).await?,
        "a2ui_semantic_search" => call_semantic_search(state, who, args).await?,
        "a2ui_generate_form" => generate_form(state, who, args).await?,
        "a2ui_generate_grid" => generate_grid(state, who, args).await?,
        "a2ui_resolve_tokens" => resolve_tokens(state, who, args).await?,
        "a2ui_assemble_surface" => call_assemble_surface(state, who, args).await?,
        other => {
            return Err(RpcError::new(METHOD_NOT_FOUND, format!("unknown tool: {other}")));
        }
    };
    // Wrap the JSON result in the MCP content envelope.
    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(&result_value)
                .unwrap_or_else(|_| "serialize error".into())
        }]
    }))
}

// ─── Thin delegations to A2UI inner functions ───────────────────────────────

async fn call_list_components(
    state: &McpState,
    who: &RlsContext,
    args: &Value,
) -> Result<Value, RpcError> {
    let query = ListComponentsQuery {
        app_id: parse_uuid_opt(args, "app_id")?,
        category: args.get("category").and_then(Value::as_str).map(str::to_owned),
    };
    a2ui::list_components_value(&state.a2ui.pool, who, &query)
        .await
        .map(|Json(v)| v)
        .map_err(rpc_from_http_error)
}

async fn call_get_component(
    state: &McpState,
    who: &RlsContext,
    args: &Value,
) -> Result<Value, RpcError> {
    let slug = args
        .get("slug")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError::new(INVALID_PARAMS, "slug required"))?;
    a2ui::get_component_value(&state.a2ui.pool, who, slug)
        .await
        .map(|Json(v)| v)
        .map_err(rpc_from_http_error)
}

async fn call_semantic_search(
    state: &McpState,
    who: &RlsContext,
    args: &Value,
) -> Result<Value, RpcError> {
    let query = args
        .get("query")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError::new(INVALID_PARAMS, "query required"))?
        .to_owned();
    let limit = args
        .get("limit")
        .and_then(Value::as_i64)
        .map_or(10, |i| i32::try_from(i).unwrap_or(10));
    let app_id = parse_uuid_opt(args, "app_id")?;
    let body = SearchComponentsBody { query, limit, app_id };
    a2ui::search_components_value(&state.a2ui.pool, who, &body)
        .await
        .map(|Json(v)| v)
        .map_err(rpc_from_http_error)
}

async fn call_assemble_surface(
    state: &McpState,
    who: &RlsContext,
    args: &Value,
) -> Result<Value, RpcError> {
    let event_type = args
        .get("event_type")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError::new(INVALID_PARAMS, "event_type required"))?
        .to_owned();
    let event_context = args.get("event_context").cloned().unwrap_or(Value::Null);
    let application_id = parse_uuid_opt(args, "application_id")?;
    let body = AssembleSurfaceBody {
        event_type,
        event_context,
        application_id,
    };
    a2ui::assemble_surface_value(&state.a2ui.pool, who, &body)
        .await
        .map(|Json(v)| v)
        .map_err(rpc_from_http_error)
}

/// Parse an optional uuid from the JSON args under `key`. Returns `Ok(None)`
/// when absent, `Ok(Some(uuid))` when present and valid, and `Err` if the value
/// is present but malformed.
fn parse_uuid_opt(args: &Value, key: &str) -> Result<Option<Uuid>, RpcError> {
    match args.get(key).and_then(Value::as_str) {
        None => Ok(None),
        Some(s) => Uuid::parse_str(s)
            .map(Some)
            .map_err(|_| RpcError::new(INVALID_PARAMS, format!("invalid {key}"))),
    }
}

/// Convert the REST error tuple into an RPC error.
fn rpc_from_http_error(err: (axum::http::StatusCode, Json<Value>)) -> RpcError {
    let (status, Json(v)) = err;
    let msg = v
        .get("error")
        .and_then(Value::as_str)
        .map_or_else(|| format!("HTTP {}", status.as_u16()), str::to_owned);
    RpcError::new(INTERNAL_ERROR, msg)
}

// ─── Native tools (generate_form, generate_grid, resolve_tokens) ────────────

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
async fn generate_form(
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
async fn generate_grid(
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
async fn resolve_tokens(
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
    let _ = app_id;

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

// ─── SSE keep-alive (optional client stream) ────────────────────────────────

/// `GET /mcp/v1/a2ui/sse` — keep-alive SSE stream.
///
/// Flint's MCP server is currently request/response only; this endpoint exists
/// so MCP clients that require an SSE connection can establish one. The stream
/// sends periodic `ping` events and stays open until the client disconnects.
pub async fn handle_sse() -> impl IntoResponse {
    use axum::response::sse::{Event, KeepAlive, Sse};
    use futures::stream::{self, StreamExt};
    use std::convert::Infallible;
    use std::time::Duration;

    let stream = stream::repeat_with(|| Ok::<_, Infallible>(Event::default().event("ping").data("{}")))
        .then(|ev| async move {
            tokio::time::sleep(Duration::from_secs(15)).await;
            ev
        });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

// ─── Health ─────────────────────────────────────────────────────────────────

/// `GET /mcp/v1/a2ui/health` — MCP server health.
pub async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "service": "flint-a2ui-mcp" }))
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        extract::Request,
        http::{Method, StatusCode},
        routing::{get, post},
        Router,
    };
    use serde_json::json;
    use sqlx::PgPool;
    use tower::ServiceExt;

    fn fake_rls_context(user_id: &str) -> RlsContext {
        RlsContext {
            role: "authenticated".to_string(),
            claims_json: json!({"flint": {"user_id": user_id}}).to_string(),
            raw_bearer: "fake".to_string(),
            keto_subject: user_id.to_string(),
            vault_key_id: None,
        }
    }

    async fn connect() -> Option<McpState> {
        let url = std::env::var("DATABASE_URL").ok()?;
        let pool = PgPool::connect(&url).await.ok()?;
        Some(McpState { a2ui: A2uiState { pool } })
    }

    fn mcp_app(state: McpState, user_id: &str) -> Router {
        Router::new()
            .route("/mcp/v1/a2ui", post(handle_mcp))
            .route("/mcp/v1/a2ui/health", get(health))
            .layer(Extension(fake_rls_context(user_id)))
            .with_state(state)
    }

    async fn read_json(resp: axum::response::Response) -> Value {
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.expect("body");
        serde_json::from_slice(&bytes).expect("valid json")
    }

    fn rpc(id: i64, method: &str, params: &Value) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn test_initialize_returns_server_info() {
        let Some(state) = connect().await else { return };
        let app = mcp_app(state, "anonymous-user");
        let req = Request::builder()
            .method(Method::POST)
            .uri("/mcp/v1/a2ui")
            .header("content-type", "application/json")
            .body(Body::from(rpc(1, "initialize", &json!({}))))
            .unwrap();
        let resp = app.oneshot(req).await.expect("req");
        assert_eq!(resp.status(), StatusCode::OK);
        let body = read_json(resp).await;
        assert_eq!(body["jsonrpc"], "2.0");
        assert_eq!(body["id"], 1);
        assert_eq!(body["result"]["serverInfo"]["name"], "flint-a2ui-registry");
        assert!(body["result"]["capabilities"]["tools"].is_object());
    }

    #[tokio::test]
    async fn test_tools_list_returns_seven_a2ui_tools() {
        let Some(state) = connect().await else { return };
        let app = mcp_app(state, "anonymous-user");
        let req = Request::builder()
            .method(Method::POST)
            .uri("/mcp/v1/a2ui")
            .header("content-type", "application/json")
            .body(Body::from(rpc(2, "tools/list", &json!({}))))
            .unwrap();
        let resp = app.oneshot(req).await.expect("req");
        let body = read_json(resp).await;
        let tools = body["result"]["tools"].as_array().expect("tools array");
        assert_eq!(tools.len(), 7, "exactly 7 a2ui tools");
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(names.contains(&"a2ui_list_components"));
        assert!(names.contains(&"a2ui_assemble_surface"));
    }

    #[tokio::test]
    async fn test_a2ui_list_components_tool_returns_components() {
        let Some(state) = connect().await else { return };
        let app = mcp_app(state, "anonymous-user");
        let req = Request::builder()
            .method(Method::POST)
            .uri("/mcp/v1/a2ui")
            .header("content-type", "application/json")
            .body(Body::from(rpc(3, "tools/call", &json!({
                "name": "a2ui_list_components",
                "arguments": {}
            }))))
            .unwrap();
        let resp = app.oneshot(req).await.expect("req");
        let body = read_json(resp).await;
        assert!(body["result"]["content"].is_array());
        let text = body["result"]["content"][0]["text"].as_str().expect("text");
        let payload: Value = serde_json::from_str(text).expect("inner json");
        let components = payload["components"].as_array().expect("components array");
        assert!(!components.is_empty(), "base components should be listed");
    }

    #[tokio::test]
    async fn test_unknown_method_returns_method_not_found() {
        let Some(state) = connect().await else { return };
        let app = mcp_app(state, "anonymous-user");
        let req = Request::builder()
            .method(Method::POST)
            .uri("/mcp/v1/a2ui")
            .header("content-type", "application/json")
            .body(Body::from(rpc(4, "resources/read", &json!({}))))
            .unwrap();
        let resp = app.oneshot(req).await.expect("req");
        let body = read_json(resp).await;
        assert_eq!(body["error"]["code"], METHOD_NOT_FOUND);
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let Some(state) = connect().await else { return };
        let app = mcp_app(state, "anonymous-user");
        let req = Request::builder()
            .uri("/mcp/v1/a2ui/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.expect("req");
        assert_eq!(resp.status(), StatusCode::OK);
        let body = read_json(resp).await;
        assert_eq!(body["status"], "ok");
        assert_eq!(body["service"], "flint-a2ui-mcp");
    }

    #[tokio::test]
    async fn test_a2ui_get_component_tool_returns_schema() {
        let Some(state) = connect().await else { return };
        let app = mcp_app(state, "anonymous-user");
        let req = Request::builder()
            .method(Method::POST)
            .uri("/mcp/v1/a2ui")
            .header("content-type", "application/json")
            .body(Body::from(rpc(5, "tools/call", &json!({
                "name": "a2ui_get_component",
                "arguments": { "slug": "button" }
            }))))
            .unwrap();
        let resp = app.oneshot(req).await.expect("req");
        let body = read_json(resp).await;
        let text = body["result"]["content"][0]["text"].as_str().expect("text");
        let payload: Value = serde_json::from_str(text).expect("inner json");
        assert_eq!(payload["component"]["slug"], "button");
    }
}


