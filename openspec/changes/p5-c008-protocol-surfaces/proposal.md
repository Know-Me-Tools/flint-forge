# p5-c008 — Protocol Surfaces: A2A Tasks + MCP Tools for Registry

**Phase:** 5 — Flint A2UI Component Registry  
**Priority:** P2 (requires Phase 7 MCP server endpoint first)  
**Depends on:** p5-c006 (REST API), Phase 7 p7-c008 (MCP server endpoint)  
**Blocks:** nothing in Phase 5; enables agent-native registry access

---

## What this change delivers

A2A task definitions and MCP tools that expose the A2UI component registry to agents — making Flint components discoverable by LLMs via tool calling.

### A2A task definitions

```yaml
# Registered as flint-platform-agent task handlers
tasks:
  - name: a2ui.component.discover
    description: "Find a UI component by natural language description"
    input:
      query: string
      application_id: uuid (optional)
    output:
      components: [{ slug, category, primitive_type, description }]

  - name: a2ui.component.assemble
    description: "Assemble an A2UI surface from an event context"
    input:
      event_type: string
      event_payload: object
      application_id: uuid
    output:
      surface: A2uiSurface

  - name: a2ui.search.semantic
    description: "Semantic vector search for UI components"
    input:
      query: string
      limit: int (default 10)
    output:
      results: [{ slug, similarity, category }]
```

### MCP tools served at `/mcp/v1/a2ui`

```json
[
  { "name": "a2ui_list_components",   "description": "List available UI components for an application" },
  { "name": "a2ui_get_component",     "description": "Get a specific component by slug" },
  { "name": "a2ui_semantic_search",   "description": "Find components by natural language description" },
  { "name": "a2ui_generate_form",     "description": "Generate a form component for a database table" },
  { "name": "a2ui_generate_grid",     "description": "Generate a data grid for a database table" },
  { "name": "a2ui_resolve_tokens",    "description": "Resolve design tokens for an application and component" },
  { "name": "a2ui_assemble_surface",  "description": "Assemble an A2UI surface from an event and context" }
]
```

These tools delegate to the `fdb-gateway` REST endpoints (p5-c006) via internal HTTP calls, not direct DB access. This keeps the MCP tool layer thin.
