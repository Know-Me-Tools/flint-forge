//! SSE keep-alive stream — optional client transport for MCP clients that
//! require an SSE connection.

use axum::response::IntoResponse;

/// `GET /mcp/v1/a2ui/sse` — keep-alive SSE stream.
///
/// Flint's MCP server is currently request/response only; this endpoint exists
/// so MCP clients that require an SSE connection can establish one. The stream
/// sends periodic `ping` events and stays open until the client disconnects.
pub async fn handle_sse() -> impl IntoResponse {
    use axum::response::sse::{Event, KeepAlive, Sse};
    use futures::stream::{self, StreamExt};
    use std::convert::Infallible;
    use std::time::Duration;

    let stream =
        stream::repeat_with(|| Ok::<_, Infallible>(Event::default().event("ping").data("{}")))
            .then(|ev| async move {
                tokio::time::sleep(Duration::from_secs(15)).await;
                ev
            });
    Sse::new(stream).keep_alive(KeepAlive::default())
}
