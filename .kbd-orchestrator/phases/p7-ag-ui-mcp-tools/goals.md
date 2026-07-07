# Goals — p7-ag-ui-mcp-tools

## Phase Summary

Deliver the AG-UI event streaming protocol, MCP tool compiler (DatabaseModel → typed tool definitions), and A2UI surface emission layer. Agents can stream lifecycle/text/tool-call events to frontends via SSE, discover database tools via MCP JSON-RPC, and receive assembled A2UI surfaces as AG-UI Custom events.

## Changes (8 total)

### P0 — Must ship

- **G1 — p7-c003-agui-emitter:** AG-UI event types + SSE endpoint `/agents/v1/<run-id>/events` in fdb-reflection/fdb-gateway. Emits RunStarted, TextMessage*, ToolCall*, StateSnapshot, RunFinished, RunError.

- **G2 — p7-c004-mcp-compiler:** `McpCompiler` in fdb-reflection: `DatabaseModel` → `Vec<McpToolDef>`. Per-table CRUD tools + per-function call tools. Integrated into `CompiledState`. Served at `/mcp/v1/tools`.

- **G3 — p7-c008-mcp-server-endpoint:** MCP JSON-RPC 2.0 server at `/mcp/v1`. **DONE** — implemented in p5-c008 (routes/mcp.rs handles `/mcp/v1/a2ui`; extend to `/mcp/v1/tools`).

### P1 — Should ship

- **G4 — p7-c005-a2ui-surface-emitter:** A2UI surface emission via AG-UI `Custom` events (type `"a2ui:surface"`). Uses Phase 5 assembler.

- **G5 — p7-c005a-copilotkit-catalog-endpoint:** `GET /a2ui/v1/catalog/:id` — **DONE** (implemented in p5-c006 routes/a2ui.rs).

- **G6 — p7-c006-a2ui-gate:** SSE processor: filter AG-UI events by Cedar scope.

- **G7 — p7-c007-agui-state-propagation:** StateManager emits StateSnapshot/StateDelta on hot-swap.

### P2 — Blocked on external deps

- **p7-c001-webhook-kiln-wiring:** Blocked on Phase 6 Kiln.
- **p7-c002-agentproto-pipe:** Blocked on FRF Phase 5 agentproto.

## Phase Complete When (MVP gate)
- [ ] `McpCompiler` generates typed MCP tool definitions from `DatabaseModel`
- [ ] `/mcp/v1/tools` serves the compiled tool list via JSON-RPC
- [ ] AG-UI SSE endpoint streams lifecycle events
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean
