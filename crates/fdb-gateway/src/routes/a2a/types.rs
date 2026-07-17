//! A2A protocol wire types: JSON-RPC envelope, error shape, task states, and
//! the standard JSON-RPC / A2A error codes.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 request envelope (same shape as MCP).
#[derive(Debug, Deserialize)]
pub struct A2aRequest {
    #[allow(dead_code)]
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

/// A2A error object.
#[derive(Debug, Serialize)]
pub struct A2aError {
    pub code: i32,
    pub message: String,
}

impl A2aError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// A2A task states (subset of the spec's TaskState enum).
#[allow(dead_code)]
pub enum TaskState {
    Submitted,
    Working,
    InputRequired,
    Completed,
    Canceled,
    Failed,
}

impl TaskState {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            TaskState::Submitted => "submitted",
            TaskState::Working => "working",
            TaskState::InputRequired => "input-required",
            TaskState::Completed => "completed",
            TaskState::Canceled => "canceled",
            TaskState::Failed => "failed",
        }
    }
}

#[allow(dead_code)]
pub(crate) const PARSE_ERROR: i32 = -32700;
pub(crate) const METHOD_NOT_FOUND: i32 = -32601;
pub(crate) const INVALID_PARAMS: i32 = -32602;
pub(crate) const INTERNAL_ERROR: i32 = -32603;
pub(crate) const TASK_NOT_FOUND: i32 = -32001;
