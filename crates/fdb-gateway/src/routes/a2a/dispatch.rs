//! A2A JSON-RPC 2.0 dispatch entry point and method routing.

use axum::{extract::State, response::Json, Extension};
use forge_identity::RlsContext;
use serde_json::{json, Value};
use uuid::Uuid;

use super::tasks::{dispatch_task, task_list};
use super::types::{A2aError, A2aRequest, INVALID_PARAMS, METHOD_NOT_FOUND};
use super::A2aState;

/// `POST /a2a/v1` — A2A JSON-RPC 2.0 dispatch.
pub async fn handle_a2a(
    State(state): State<A2aState>,
    Extension(who): Extension<RlsContext>,
    Json(req): Json<A2aRequest>,
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

async fn dispatch(
    state: &A2aState,
    who: &RlsContext,
    method: &str,
    params: Option<&Value>,
) -> Result<Value, A2aError> {
    match method {
        "tasks/list" => Ok(json!({ "tasks": task_list() })),
        "tasks/send" => {
            let task_name = params
                .and_then(|p| p.get("task"))
                .and_then(|t| t.get("name"))
                .and_then(Value::as_str)
                .ok_or_else(|| A2aError::new(INVALID_PARAMS, "task.name required"))?;
            let task_input = params
                .and_then(|p| p.get("task"))
                .and_then(|t| t.get("input"))
                .unwrap_or(&Value::Null);
            let task_id = params
                .and_then(|p| p.get("task"))
                .and_then(|t| t.get("id"))
                .and_then(Value::as_str)
                .map_or_else(|| Uuid::new_v4().to_string(), str::to_owned);
            dispatch_task(state, who, &task_id, task_name, task_input).await
        }
        _ => Err(A2aError::new(
            METHOD_NOT_FOUND,
            format!("unknown method: {method}"),
        )),
    }
}
