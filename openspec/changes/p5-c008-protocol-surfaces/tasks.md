# p5-c008 Tasks — Protocol Surfaces

## Tasks

- [ ] Register A2A task handlers in `fdb-gateway`: `a2ui.component.discover`, `a2ui.component.assemble`, `a2ui.search.semantic`
- [ ] Add MCP tool definitions to `fdb-gateway` MCP compiler: 7 a2ui tools
- [ ] Wire MCP a2ui tools to delegate to REST endpoints (p5-c006) via internal HTTP or direct function calls
- [ ] Expose MCP a2ui tools at `/mcp/v1/a2ui` namespace (separate from DB schema tools at `/mcp/v1/tools`)
- [ ] Gate test: Claude Desktop (or test MCP client) can call `a2ui_list_components` and receive base components
- [ ] Gate test: `a2ui_generate_form` for `public.orders` table returns a Form component with auto-generated fields
