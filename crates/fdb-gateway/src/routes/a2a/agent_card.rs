//! Agent Card endpoint — A2A capability/skill discovery document.

use axum::response::Json;
use serde_json::{json, Value};

/// `GET /.well-known/agent.json` — Agent Card per the A2A spec.
///
/// Describes the Flint A2UI Registry agent, its capabilities, and the three
/// skills it exposes. Agents use this to discover what tasks they can delegate.
pub async fn agent_card() -> Json<Value> {
    Json(json!({
        "name": "flint-a2ui-registry",
        "description": "Flint Forge A2UI Component Registry — discovers and assembles UI components for LLM agents.",
        "version": env!("CARGO_PKG_VERSION"),
        "protocolVersion": "0.1",
        "url": "/a2a/v1",
        "capabilities": {
            "taskPush": true,
            "taskPull": false,
            "streaming": false
        },
        "skills": [
            {
                "id": "a2ui.component.discover",
                "name": "Discover UI Component",
                "description": "Find a UI component by natural language description. Returns matching components with slug, category, and description.",
                "inputSchema": {
                    "type": "object",
                    "required": ["query"],
                    "properties": {
                        "query":      { "type": "string", "description": "Natural language description of the desired component" },
                        "limit":      { "type": "integer", "minimum": 1, "maximum": 50, "default": 10 },
                        "app_id":     { "type": "string", "format": "uuid" }
                    }
                }
            },
            {
                "id": "a2ui.component.assemble",
                "name": "Assemble A2UI Surface",
                "description": "Assemble an A2UI surface from an event context. Applies application-specific assembly rules and falls back to default table bindings.",
                "inputSchema": {
                    "type": "object",
                    "required": ["event_type"],
                    "properties": {
                        "event_type":     { "type": "string", "description": "Event name driving the assembly (e.g. 'mount', 'record.select')" },
                        "event_payload":  { "type": "object", "description": "Event payload (table, record id, etc.)" },
                        "application_id": { "type": "string", "format": "uuid" }
                    }
                }
            },
            {
                "id": "a2ui.search.semantic",
                "name": "Semantic Search Components",
                "description": "Semantic vector search for UI components using hybrid text + embedding similarity.",
                "inputSchema": {
                    "type": "object",
                    "required": ["query"],
                    "properties": {
                        "query":    { "type": "string", "description": "Natural language description of the desired component" },
                        "limit":    { "type": "integer", "minimum": 1, "maximum": 50, "default": 10 },
                        "app_id":   { "type": "string", "format": "uuid" }
                    }
                }
            }
        ]
    }))
}
