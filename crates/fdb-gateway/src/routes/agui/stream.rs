//! SSE endpoint — `GET /agents/v1/:run_id/events`.

use axum::{
    extract::{Path, State},
    response::sse::{Event, KeepAlive, Sse},
    response::IntoResponse,
    Json,
};
use futures::stream::StreamExt;
use serde_json::json;

use fdb_domain::AgUiEvent;

use super::state::AgUiState;

/// `GET /agents/v1/:run_id/events` — SSE stream of AG-UI events for a run.
///
/// Streams events until a terminal event (`RunFinished`/`RunError`) or the
/// client disconnects. If the run doesn't exist yet (lazy creation), the
/// stream opens immediately and waits for the first event.
pub async fn stream_events(
    State(state): State<AgUiState>,
    Path(run_id): Path<String>,
) -> impl IntoResponse {
    // Ensure the channel exists (lazy creation — allows subscribing before publish).
    let _ = state.channel_for(&run_id).await;

    let Some(event_stream) = state.subscribe(&run_id).await else {
        return (
            axum::http::StatusCode::NOT_FOUND,
            Json(json!({"error": "run not found"})),
        )
            .into_response();
    };

    // Map AgUiEvent → SSE Event, terminating on RunFinished/RunError.
    let sse_stream = event_stream.take_while(|event| {
        let is_terminal = event.is_terminal();
        async move { !is_terminal }
    });

    let mapped = sse_stream.map(|event: AgUiEvent| {
        let event_type = match &event {
            AgUiEvent::RunStarted { .. } => "RunStarted",
            AgUiEvent::TextMessageStart { .. } => "TextMessageStart",
            AgUiEvent::TextMessageContent { .. } => "TextMessageContent",
            AgUiEvent::TextMessageEnd { .. } => "TextMessageEnd",
            AgUiEvent::ToolCallStart { .. } => "ToolCallStart",
            AgUiEvent::ToolCallArgs { .. } => "ToolCallArgs",
            AgUiEvent::ToolCallEnd { .. } => "ToolCallEnd",
            AgUiEvent::ToolCallResult { .. } => "ToolCallResult",
            AgUiEvent::StateSnapshot { .. } => "StateSnapshot",
            AgUiEvent::StateDelta { .. } => "StateDelta",
            AgUiEvent::Custom { .. } => "Custom",
            AgUiEvent::RunFinished { .. } => "RunFinished",
            AgUiEvent::RunError { .. } => "RunError",
            // Forward-compatibility guard: new AG-UI event variants added in future
            // versions of fdb-domain will be serialised with their JSON type tag.
            _ => "Unknown",
        };
        let data = serde_json::to_string(&event).unwrap_or_else(|_| "{}".into());
        Ok::<_, std::convert::Infallible>(Event::default().event(event_type).data(data))
    });

    // Send the terminal event as the final SSE message before closing.
    Sse::new(mapped)
        .keep_alive(KeepAlive::default())
        .into_response()
}
