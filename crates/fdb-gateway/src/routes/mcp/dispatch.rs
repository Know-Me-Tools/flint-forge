//! JSON-RPC method dispatch and MCP tool-call routing.
//!
//! `dispatch` resolves the top-level JSON-RPC method (`initialize`, `ping`,
//! `tools/list`, `tools/call`). `dispatch_tool` validates the tool name
//! against the fixed set defined in [`super::tools::tool_definitions`] before
//! invoking its handler — any name outside that set is rejected with
//! `METHOD_NOT_FOUND`. This validation is security-relevant (every tool call
//! runs under the caller's RLS context) and must not be weakened.

use axum::Json;
use forge_identity::RlsContext;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::routes::a2ui::{self, AssembleSurfaceBody, ListComponentsQuery, SearchComponentsBody};

use super::native_tools::{generate_form, generate_grid, resolve_tokens};
use super::protocol::{RpcError, INTERNAL_ERROR, INVALID_PARAMS, METHOD_NOT_FOUND};
use super::tools::tool_definitions;
use super::McpState;

/// Method dispatch. Returns a JSON-RPC `result` value or an error.
pub(super) async fn dispatch(
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
        _ => Err(RpcError::new(
            METHOD_NOT_FOUND,
            format!("unknown method: {method}"),
        )),
    }
}

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
            return Err(RpcError::new(
                METHOD_NOT_FOUND,
                format!("unknown tool: {other}"),
            ));
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
        category: args
            .get("category")
            .and_then(Value::as_str)
            .map(str::to_owned),
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
    let body = SearchComponentsBody {
        query,
        limit,
        app_id,
    };
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
