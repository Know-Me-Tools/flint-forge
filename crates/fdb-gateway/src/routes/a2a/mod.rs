//! A2A (Agent-to-Agent) protocol surface — task handler registry.
//!
//! Exposes three A2UI tasks via the A2A JSON-RPC protocol so agents built on
//! the A2A spec (Google A2A v0.1+) can discover and assemble Flint components.
//!
//! # Endpoints
//!
//! - `GET  /.well-known/agent.json` — Agent Card describing capabilities + skills
//! - `POST /a2a/v1` — JSON-RPC 2.0 dispatch (`tasks/send`, `tasks/list`)
//!
//! # Security
//!
//! Mounted behind `rls_layer::require_rls`; every task call runs under the
//! caller's verified `RlsContext`. Tasks delegate to the same inner functions
//! as the REST + MCP surfaces — single SQL authority.
#![forbid(unsafe_code)]

mod agent_card;
mod dispatch;
mod helpers;
mod tasks;
mod types;

#[cfg(test)]
mod tests;

pub use agent_card::agent_card;
pub use dispatch::handle_a2a;

use crate::routes::a2ui::A2uiState;

/// A2A server-scoped state. Wraps the A2UI route state so task handlers can
/// call the shared inner functions.
#[derive(Clone)]
pub struct A2aState {
    pub a2ui: A2uiState,
}
