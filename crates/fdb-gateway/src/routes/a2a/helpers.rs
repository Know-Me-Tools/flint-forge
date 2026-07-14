//! Small shared helpers for A2A task handlers.

use axum::response::Json;
use serde_json::Value;
use uuid::Uuid;

use super::types::{A2aError, INTERNAL_ERROR, INVALID_PARAMS};

/// Parse an optional uuid from the JSON args under `key`.
pub(super) fn parse_uuid_opt(input: &Value, key: &str) -> Result<Option<Uuid>, A2aError> {
    match input.get(key).and_then(Value::as_str) {
        None => Ok(None),
        Some(s) => Uuid::parse_str(s)
            .map(Some)
            .map_err(|_| A2aError::new(INVALID_PARAMS, format!("invalid {key}"))),
    }
}

/// Convert the REST error tuple into an A2A error.
pub(super) fn http_to_a2a_error(err: (axum::http::StatusCode, Json<Value>)) -> A2aError {
    let (status, Json(v)) = err;
    let msg = v
        .get("error")
        .and_then(Value::as_str)
        .map_or_else(|| format!("HTTP {}", status.as_u16()), str::to_owned);
    A2aError::new(INTERNAL_ERROR, msg)
}
