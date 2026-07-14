//! AG-UI event emitter — SSE streaming endpoint for agent run events.
//!
//! Implements the AG-UI protocol v0.1 event streaming surface. Agent execution
//! paths (Kiln WASM functions, flint_hooks targets) publish events to a per-run
//! broadcast channel; frontends subscribe via SSE at
//! `/agents/v1/<run-id>/events`.
//!
//! # Events
//!
//! Lifecycle: `RunStarted`, `RunFinished`, `RunError`
//! Text: `TextMessageStart`, `TextMessageContent`, `TextMessageEnd`
//! Tool calls: `ToolCallStart`, `ToolCallArgs`, `ToolCallEnd`, `ToolCallResult`
//! State: `StateSnapshot`, `StateDelta`
//! Custom: `Custom` (used for `"a2ui:surface"` delivery)
//!
//! # Architecture
//!
//! Each run gets a `broadcast::Sender<AgUiEvent>`. Events are buffered in a
//! `BroadcastStream` and converted to SSE `Event`s. The stream stays open until
//! a terminal event (`RunFinished`/`RunError`) or the client disconnects.
#![forbid(unsafe_code)]

mod publish;
mod state;
mod stream;
mod surface;

#[cfg(test)]
mod tests;

pub use publish::{publish_event, start_run};
pub use state::AgUiState;
pub use stream::stream_events;
pub use surface::assemble_and_emit_surface;
