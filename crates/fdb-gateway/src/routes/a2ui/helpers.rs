//! Shared helpers for A2UI route handlers (claims extraction and error
//! mapping). Kept separate so every handler module can depend on them
//! without duplicating logic.

use axum::{http::StatusCode, response::Json};
use forge_identity::RlsContext;
use serde_json::{json, Value};

/// Extract `flint.user_id` from the `RlsContext` claims string.
pub(super) fn user_id_from_claims(who: &RlsContext) -> Option<String> {
    claims_json(who)
        .get("flint")
        .and_then(|v| v.get("user_id"))
        .and_then(|v| v.as_str())
        .map(String::from)
}

/// Build a JSON object from the `RlsContext` claims string.
pub(super) fn claims_json(who: &RlsContext) -> Value {
    serde_json::from_str(&who.claims_json).unwrap_or(Value::Null)
}

pub(super) fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, Json<Value>) {
    tracing::error!(error = %err, "a2ui api error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "internal server error" })),
    )
}
