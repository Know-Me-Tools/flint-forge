//! JSON-RPC 2.0 protocol types and error codes shared by the MCP dispatch
//! layer (`super::dispatch`) and the top-level route handlers (`super`).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 request envelope.
///
/// `pub(crate)` (rather than `pub(super)`) because it appears as a type
/// parameter of the `Handler` bound satisfied by `handle_mcp`, which is
/// registered from `main.rs` outside the `mcp` module tree.
#[derive(Debug, Deserialize)]
pub(crate) struct McpRequest {
    #[allow(dead_code)]
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Serialize)]
pub(super) struct RpcError {
    pub code: i32,
    pub message: String,
}

impl RpcError {
    pub(super) fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// Standard JSON-RPC error codes.
pub(super) const PARSE_ERROR: i32 = -32700;
pub(super) const METHOD_NOT_FOUND: i32 = -32601;
pub(super) const INVALID_PARAMS: i32 = -32602;
pub(super) const INTERNAL_ERROR: i32 = -32603;

/// Silence the dead-code lint for `PARSE_ERROR` — it is part of the protocol
/// surface but not currently returned (axum rejects malformed JSON before this
/// handler runs).
#[allow(dead_code)]
pub(super) fn parse_error_marker() -> i32 {
    PARSE_ERROR
}
