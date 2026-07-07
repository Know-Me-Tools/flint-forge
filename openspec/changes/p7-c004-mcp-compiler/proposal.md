# p7-c004 — MCP Tools Compiler

**Phase:** 7 — AG-UI Integration + MCP Tools Compiler
**Priority:** P0
**Depends on:** p2-c003 (CompiledState struct) — delivered
**Blocks:** p7-c008 (MCP server endpoint `/mcp/v1/tools`)

---

## What this change delivers

Replaces the `todo!()` stub in `fdb-reflection/src/compilers/mcp.rs` with a real compiler that transforms `DatabaseModel` into typed MCP tool definitions.

### Per-table tools (5 per table)

| Tool | Description |
|---|---|
| `list_<table>` | List rows with `select`, `eq`, `order`, `limit`, `offset` |
| `get_<table>` | Get a single row by primary key |
| `create_<table>` | Insert a new row |
| `update_<table>` | Update a row by primary key |
| `delete_<table>` | Delete a row by primary key |

### Per-function tools (1 per function)

| Tool | Description |
|---|---|
| `call_<function>` | Call a Postgres function with typed args |

### Per-view tools (1 per view)

| Tool | Description |
|---|---|
| `list_<view>` | List rows from a view (read-only) |

### Schema filtering

Internal schemas (`flint_meta`, `flint_a2ui`, `auth`, `graphql_public`, `_flint`) are excluded by default. A `schemas` parameter can scope the output.

### Integration

- `McpCompiler::compile(&model)` returns a `serde_json::Value` containing the `tools` array
- Added to `CompiledState` as `mcp_tools_doc: Value`
- Hot-swapped on DDL changes via `StateManager::do_compile()`
- Served at `/mcp/v1/tools` via the existing MCP server in `fdb-gateway`
