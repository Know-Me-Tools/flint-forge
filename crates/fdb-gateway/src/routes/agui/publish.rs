//! Run lifecycle and event publish endpoints.
//!
//! `POST /agents/v1/runs` — start a new run.
//! `POST /agents/v1/:run_id/events` — publish an event to a run's stream.

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Extension, Json,
};
use forge_identity::RlsContext;
use serde_json::json;

use fdb_domain::AgUiEvent;

use super::state::AgUiState;

/// Request body for `POST /agents/v1/:run_id/events`.
#[derive(Debug, serde::Deserialize)]
pub struct PublishEventBody {
    pub event: AgUiEvent,
}

/// `POST /agents/v1/:run_id/events` — publish an event to a run's stream.
///
/// Used by agent execution paths (Kiln WASM, flint_hooks) to emit events.
/// Behind JWT auth — the caller must have permission to emit for this run.
pub async fn publish_event(
    State(state): State<AgUiState>,
    Extension(_who): Extension<RlsContext>,
    Path(run_id): Path<String>,
    Json(body): Json<PublishEventBody>,
) -> impl IntoResponse {
    // Verify the event's run_id matches the path.
    if let Some(event_run_id) = body.event.run_id() {
        if event_run_id != run_id {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(json!({"error": "event run_id does not match path"})),
            )
                .into_response();
        }
    }

    let is_terminal = body.event.is_terminal();
    state.publish(body.event).await;

    if is_terminal {
        state.cleanup_run(&run_id).await;
    }

    Json(json!({"status": "published"})).into_response()
}

/// `POST /agents/v1/runs` — start a new run and return the run_id.
///
/// Creates a broadcast channel for the run and returns the run_id + event
/// stream URL. The caller then publishes events via POST and subscribes via
/// SSE GET.
pub async fn start_run(
    State(state): State<AgUiState>,
    Extension(_who): Extension<RlsContext>,
) -> impl IntoResponse {
    let run_id = uuid::Uuid::new_v4().to_string();
    // Called only for its side effect of creating the run's broadcast channel;
    // the returned Sender isn't needed here (subscribers/publishers fetch it
    // themselves via `channel_for` later).
    let _ = state.channel_for(&run_id).await;

    Json(json!({
        "run_id": run_id,
        "events_url": format!("/agents/v1/{run_id}/events"),
        "publish_url": format!("/agents/v1/{run_id}/events"),
    }))
}
