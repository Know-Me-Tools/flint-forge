//! Context and message types for the A2UI assembler.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

/// All inputs needed to assemble a surface.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssemblyContext {
    /// Event type name, e.g. `tool_call_completed`.
    pub event_type: String,
    /// Event payload. Default binding resolution looks for `data_source.schema`
    /// and `data_source.table`.
    pub event_payload: Value,
    /// Application that should own the assembled surface.
    pub application_id: Option<Uuid>,
    /// JWT claims of the caller, used for any permission-filtered resolution.
    pub jwt_claims: Value,
    /// Optional explicit surface id; otherwise a new UUID is generated.
    pub surface_id: Option<Uuid>,
}

/// A fully assembled A2UI surface, represented as a sequence of messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2uiSurface {
    pub surface_id: Uuid,
    pub catalog_id: String,
    pub messages: Vec<A2uiMessage>,
}

impl A2uiSurface {
    /// Serialize the whole surface to a JSON value.
    pub fn to_json(&self) -> Value {
        json!({
            "surfaceId": self.surface_id,
            "catalogId": self.catalog_id,
            "messages": self.messages,
        })
    }
}

/// A single A2UI message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2uiMessage {
    pub op: String,
    #[serde(flatten)]
    pub payload: Value,
}
