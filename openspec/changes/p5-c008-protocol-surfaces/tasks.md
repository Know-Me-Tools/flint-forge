# p5-c008 Tasks — Protocol Surfaces

## Tasks

- [x] Register A2A task handlers in `fdb-gateway`: `a2ui.component.discover`, `a2ui.component.assemble`, `a2ui.search.semantic`
- [x] Add MCP tool definitions to `fdb-gateway` MCP compiler: 7 a2ui tools
- [x] Wire MCP a2ui tools to delegate to REST endpoints (p5-c006) via internal HTTP or direct function calls
- [x] Expose MCP a2ui tools at `/mcp/v1/a2ui` namespace (separate from DB schema tools at `/mcp/v1/tools`)
- [ ] Gate test: Claude Desktop (or test MCP client) can call `a2ui_list_components` and receive base components
- [ ] Gate test: `a2ui_generate_form` for `public.orders` table returns a Form component with auto-generated fields

## Blocker fix (p7-c008 MCP server endpoint)

- [x] Implemented minimal MCP server endpoint at `/mcp/v1/a2ui` in `crates/fdb-gateway/src/routes/mcp.rs`
- [x] JSON-RPC 2.0 protocol: `initialize`, `tools/list`, `tools/call`, `ping`
- [x] SSE keep-alive stream at `GET /mcp/v1/a2ui/sse`
- [x] Health endpoint at `GET /mcp/v1/a2ui/health`
- [x] Route mounted behind `rls_layer::require_rls`
- [x] A2UI inner functions extracted (`*_value`) so REST + MCP share a single SQL authority
- [x] `cargo clippy -p fdb-gateway -- -D warnings` clean
- [x] 6 unit tests covering initialize, tools/list, list_components tool, get_component tool, unknown method, health
