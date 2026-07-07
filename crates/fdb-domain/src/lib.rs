//! Flint Quarry domain types.
#![forbid(unsafe_code)]

use forge_domain::Json;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableMeta {
    pub schema: String,
    pub name: String,
    pub columns: Vec<ColumnMeta>,
    pub primary_key: Vec<String>,
    pub rls_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMeta {
    pub name: String,
    pub sql_type: String,
    pub nullable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ChangeOp {
    Insert,
    Update,
    Delete,
    Upsert,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeEvent {
    pub op: ChangeOp,
    pub schema: String,
    pub table: String,
    pub record: Option<Json>,
    pub old_record: Option<Json>,
}

#[derive(Debug, Clone)]
pub struct RestQuery {
    pub schema: String,
    pub table: String,
    pub select: Option<String>,
    pub filters: Vec<(String, String, String)>, // (column, op, value)
    pub order: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct RestResult {
    pub rows: Json,
    pub count: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct GraphQlRequest {
    pub query: String,
    pub variables: Json,
    pub operation_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SubscriptionSpec {
    pub tenant: String,
    pub entity_type: String,
    pub filter: Option<Json>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaVersion(pub u64);

/// Request body for `POST /rpc/vector` — vector similarity search.
///
/// The `embedding` is matched against `column` in `table` using the pgvector
/// cosine distance operator (`<=>`). Results are returned ordered by ascending
/// distance (most similar first).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct VectorRpcRequest {
    /// Query embedding as a flat array of f32 values.
    pub embedding: Vec<f32>,
    /// Qualified table name: `"<schema>.<table>"` or just `"<table>"` (defaults to `public`).
    pub table: String,
    /// Column name holding the `vector` values.
    pub column: String,
    /// Maximum number of results to return (default 10, capped at 1000).
    #[serde(default = "default_vector_limit")]
    pub limit: u32,
    /// Optional JSON filter applied as an additional WHERE clause (future use).
    pub filter: Option<serde_json::Value>,
}

fn default_vector_limit() -> u32 {
    10
}

// ─── AG-UI Event Types ──────────────────────────────────────────────────────

/// AG-UI protocol event types (v0.1 spec).
///
/// These are the standard lifecycle, text, tool-call, and state events
/// streamed to agent frontends over SSE at `/agents/v1/<run-id>/events`.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[allow(clippy::large_enum_variant)]
pub enum AgUiEvent {
    /// Sent when a run starts.
    #[serde(rename = "RunStarted")]
    RunStarted {
        run_id: String,
        #[serde(default)]
        thread_id: Option<String>,
    },
    /// Marks the beginning of a text message from the agent.
    #[serde(rename = "TextMessageStart")]
    TextMessageStart {
        message_id: String,
        role: String,
    },
    /// A chunk of text content within a message.
    #[serde(rename = "TextMessageContent")]
    TextMessageContent {
        message_id: String,
        content: String,
    },
    /// Marks the end of a text message.
    #[serde(rename = "TextMessageEnd")]
    TextMessageEnd { message_id: String },
    /// Marks the beginning of a tool call.
    #[serde(rename = "ToolCallStart")]
    ToolCallStart {
        tool_call_id: String,
        tool_name: String,
        parent_message_id: Option<String>,
    },
    /// Arguments being streamed for a tool call (incremental JSON delta).
    #[serde(rename = "ToolCallArgs")]
    ToolCallArgs {
        tool_call_id: String,
        args: String,
    },
    /// Marks the end of a tool call (args are complete).
    #[serde(rename = "ToolCallEnd")]
    ToolCallEnd { tool_call_id: String },
    /// Result of a tool call, sent back to the agent/frontend.
    #[serde(rename = "ToolCallResult")]
    ToolCallResult {
        tool_call_id: String,
        result: serde_json::Value,
        #[serde(default)]
        error: Option<String>,
    },
    /// Full snapshot of agent state (MCP tools, catalog version, etc.).
    #[serde(rename = "StateSnapshot")]
    StateSnapshot {
        run_id: String,
        state: serde_json::Value,
    },
    /// Incremental state change (JSON Patch format).
    #[serde(rename = "StateDelta")]
    StateDelta {
        run_id: String,
        delta: Vec<serde_json::Value>,
    },
    /// Custom event — used for A2UI surface delivery (`"a2ui:surface"`).
    #[serde(rename = "Custom")]
    Custom {
        run_id: String,
        name: String,
        value: serde_json::Value,
    },
    /// Sent when a run completes successfully.
    #[serde(rename = "RunFinished")]
    RunFinished { run_id: String },
    /// Sent when a run fails.
    #[serde(rename = "RunError")]
    RunError { run_id: String, message: String },
}

impl AgUiEvent {
    /// The run_id this event belongs to, if applicable.
    pub fn run_id(&self) -> Option<&str> {
        match self {
            AgUiEvent::RunStarted { run_id, .. }
            | AgUiEvent::StateSnapshot { run_id, .. }
            | AgUiEvent::StateDelta { run_id, .. }
            | AgUiEvent::Custom { run_id, .. }
            | AgUiEvent::RunFinished { run_id, .. }
            | AgUiEvent::RunError { run_id, .. } => Some(run_id.as_str()),
            _ => None,
        }
    }

    /// True if this event terminates the SSE stream for a run.
    pub fn is_terminal(&self) -> bool {
        matches!(self, AgUiEvent::RunFinished { .. } | AgUiEvent::RunError { .. })
    }
}
