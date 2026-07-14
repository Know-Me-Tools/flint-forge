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
//!
//! # Module layout
//!
//! - [`protocol`] — JSON-RPC request/error types and error codes.
//! - [`tools`] — the `tools/list` catalog (fixed set of tool names).
//! - [`dispatch`] — JSON-RPC method dispatch and tool-name validation.
//! - [`native_tools`] — tools that query `flint_a2ui` bindings directly.
//! - [`sse`] — the keep-alive SSE stream.
#![forbid(unsafe_code)]

use axum::{extract::State, response::Json, Extension};
use forge_identity::RlsContext;
use serde_json::{json, Value};

use crate::routes::a2ui::A2uiState;

mod dispatch;
mod native_tools;
mod protocol;
mod sse;
mod tools;

#[cfg(test)]
mod tests;

use protocol::McpRequest;

pub use sse::handle_sse;

/// MCP server-scoped state. Wraps the A2UI route state so tools can call the
/// shared inner handler functions.
#[derive(Clone)]
pub struct McpState {
    pub a2ui: A2uiState,
}

/// `POST /mcp/v1/a2ui` — JSON-RPC 2.0 dispatch.
///
/// Every call shares the caller's `RlsContext` (installed by `require_rls`).
pub async fn handle_mcp(
    State(state): State<McpState>,
    Extension(who): Extension<RlsContext>,
    Json(req): Json<McpRequest>,
) -> Json<Value> {
    let result = dispatch::dispatch(&state, &who, &req.method, req.params.as_ref()).await;
    Json(match result {
        Ok(value) => json!({ "jsonrpc": "2.0", "id": req.id, "result": value }),
        Err(e) => json!({
            "jsonrpc": "2.0",
            "id": req.id,
            "error": { "code": e.code, "message": e.message }
        }),
    })
}

/// `GET /mcp/v1/a2ui/health` — MCP server health.
pub async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "service": "flint-a2ui-mcp" }))
}
