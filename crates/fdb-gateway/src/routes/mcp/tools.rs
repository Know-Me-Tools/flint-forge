//! MCP tool registry — the `tools/list` catalog of available A2UI tools.
//!
//! This is the single source of truth for the fixed set of tool names that
//! `super::dispatch::dispatch_tool` accepts. Any tool name not listed here
//! must be rejected by dispatch — see the security note there.

use serde_json::{json, Value};

/// Return the `tools/list` result — 7 A2UI tools.
pub(super) fn tool_definitions() -> Value {
    json!({
        "tools": [
            {
                "name": "a2ui_list_components",
                "description": "List available UI components for an application. Returns base components plus app-scoped components the caller can access.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "app_id":     { "type": "string", "format": "uuid", "description": "Optional application ID to include app-specific components" },
                        "category":   { "type": "string", "description": "Optional category filter (e.g. 'form', 'display')" }
                    }
                }
            },
            {
                "name": "a2ui_get_component",
                "description": "Get a specific component by slug, including its full JSON schema and render targets.",
                "inputSchema": {
                    "type": "object",
                    "required": ["slug"],
                    "properties": {
                        "slug": { "type": "string", "description": "Component slug (e.g. 'button', 'data-grid')" }
                    }
                }
            },
            {
                "name": "a2ui_semantic_search",
                "description": "Find components by natural language description using hybrid text + semantic vector search.",
                "inputSchema": {
                    "type": "object",
                    "required": ["query"],
                    "properties": {
                        "query":    { "type": "string", "description": "Natural language description of the desired component" },
                        "limit":    { "type": "integer", "minimum": 1, "maximum": 50, "default": 10 },
                        "app_id":   { "type": "string", "format": "uuid" }
                    }
                }
            },
            {
                "name": "a2ui_generate_form",
                "description": "Generate a Form component for a database table using its auto-generated and manual bindings.",
                "inputSchema": {
                    "type": "object",
                    "required": ["schema", "table"],
                    "properties": {
                        "schema": { "type": "string", "description": "Postgres schema name (e.g. 'public')" },
                        "table":  { "type": "string", "description": "Postgres table name (e.g. 'orders')" }
                    }
                }
            },
            {
                "name": "a2ui_generate_grid",
                "description": "Generate a data grid component for a database table using its bindings.",
                "inputSchema": {
                    "type": "object",
                    "required": ["schema", "table"],
                    "properties": {
                        "schema": { "type": "string" },
                        "table":  { "type": "string" }
                    }
                }
            },
            {
                "name": "a2ui_resolve_tokens",
                "description": "Resolve design tokens (color, spacing, typography) for an application and component category.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "application_slug": { "type": "string", "default": "flint-base" },
                        "category":         { "type": "string", "description": "Component category (e.g. 'form', 'display')" }
                    }
                }
            },
            {
                "name": "a2ui_assemble_surface",
                "description": "Assemble an A2UI surface from an event context. Applies application-specific assembly rules and falls back to default table bindings.",
                "inputSchema": {
                    "type": "object",
                    "required": ["event_type"],
                    "properties": {
                        "event_type":       { "type": "string", "description": "Event name driving the assembly (e.g. 'mount', 'record.select')" },
                        "event_context":    { "type": "object", "description": "Event payload (table, record id, etc.)" },
                        "application_id":   { "type": "string", "format": "uuid" }
                    }
                }
            }
        ]
    })
}
