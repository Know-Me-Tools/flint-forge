---
name: flint-ui
version: "1.0.0"
description: >
  Flint Forge UI component skill — complete reference for the A2UI component
  registry, React 19 SDK, Flutter SDK, and HTMX renderer. Enables Claude Code
  to generate correct, prop-accurate Flint UI code for all 3 rendering targets
  without hallucinating component names or API signatures.
tags: [flint, a2ui, react, flutter, htmx, ui, components]
authors: [prometheus-ags]
---

# Flint UI Skill

Use this skill whenever you are:
- Building UIs that consume Flint components (`@flint/react`, `flint_genui`, or fdb-gateway HTMX)
- Writing code that calls the A2UI registry REST API (`/a2ui/v1/*`)
- Assembling surfaces via AG-UI Custom events (`"a2ui:surface"`)
- Working with MCP tools generated from Flint's `DatabaseModel`

## Quick Reference

### Component categories (55 base components)

| Category | Slugs |
|---|---|
| **layout** (8) | `container` `row` `column` `grid` `stack` `divider` `spacer` `scroll-area` |
| **data-display** (10) | `data-grid` `data-table` `text` `badge` `tag` `avatar` `stat-card` `timeline` `code-block` `json-viewer` |
| **data-display (list)** | `list` `detail-view` |
| **input** (14) | `form` `text-input` `number-input` `select` `multi-select` `date-picker` `checkbox` `radio` `toggle` `textarea` `file-upload` `search-input` `color-picker` `slider` |
| **action** (6) | `button` `action-bar` `dropdown-menu` `context-menu` `fab` `link` |
| **navigation** (6) | `nav-bar` `sidebar` `tabs` `breadcrumb` `pagination` `stepper` |
| **feedback** (8) | `alert` `toast` `modal` `dialog` `loading-spinner` `progress-bar` `empty-state` `error-boundary` |
| **system** (1) | `flint-meta-schema` |

Full prop signatures: `catalogs/components.md`

### REST API endpoints (fdb-gateway)

```
GET    /a2ui/v1/components                     # list base components
GET    /a2ui/v1/components/:slug               # get component with schema
POST   /a2ui/v1/components/search              # hybrid text+semantic search
GET    /a2ui/v1/components/bindings/:schema/:table
GET    /a2ui/v1/applications
GET    /a2ui/v1/applications/:id
GET    /a2ui/v1/catalog/:catalog_id            # A2UI v0.9 catalog JSON
POST   /a2ui/v1/surfaces/assemble              # assemble surface from event
GET    /mcp/v1/tools                           # compiled MCP tool definitions
POST   /mcp/v1/a2ui                            # MCP JSON-RPC for A2UI tools
GET    /agents/v1/:run_id/events               # AG-UI SSE stream
POST   /agents/v1/:run_id/events               # publish AG-UI event
POST   /agents/v1/runs                         # start new run
POST   /agents/v1/:run_id/surfaces/assemble    # assemble + emit A2UI surface
```

Full API: `catalogs/react-api.md`, `catalogs/flutter-api.md`, `catalogs/htmx-api.md`

### React 19 — @flint/react

```tsx
import { FlintProvider, FlintSurface, useFlint } from '@flint/react';

// Wrap your app
<FlintProvider catalogUrl="/a2ui/v1/catalog/flint-base/1.0">
  <FlintSurface surfaceId="orders-view" runId="run-abc123" />
</FlintProvider>

// Use a component directly
import { DataGrid } from '@flint/react/components';

<DataGrid
  columns={["id","status","total"]}
  data={rows}
  onRowClick={(row) => navigate(`/orders/${row.id}`)}
  pagination={{ pageSize: 25 }}
/>
```

### Flutter — flint_genui

```dart
import 'package:flint_genui/flint_genui.dart';

FlintSurface(
  transport: FlintA2uiTransport(
    catalogUrl: 'https://api.example.com/a2ui/v1/catalog/flint-base/1.0',
    runId: 'run-abc123',
    eventsUrl: 'https://api.example.com/agents/v1/run-abc123/events',
  ),
  surfaceId: 'orders-view',
)
```

### HTMX — fdb-gateway renderer

```html
<!-- Render component with demo props -->
<div hx-get="/htmx/components/data-grid"
     hx-trigger="load"
     hx-swap="outerHTML"></div>

<!-- Render with custom props -->
<form hx-post="/htmx/components/form"
      hx-ext="json-enc"
      hx-swap="outerHTML">
  <input type="hidden" name="fields[0][name]" value="email">
  <button type="submit">Preview</button>
</form>
```

### AG-UI Custom event — A2UI surface delivery

```json
{
  "type": "Custom",
  "name": "a2ui:surface",
  "value": {
    "protocol": "a2ui/0.9",
    "catalogId": "https://api.example.com/a2ui/v1/catalog/flint-base/1.0",
    "messages": [
      { "op": "createSurface",    "surfaceId": "orders-view", "catalogId": "..." },
      { "op": "updateComponents", "surfaceId": "orders-view", "components": [...] },
      { "op": "updateDataModel",  "surfaceId": "orders-view", "path": "/data", "value": {...} }
    ]
  }
}
```

### MCP tools (auto-generated from DatabaseModel)

Each user-visible table gets 5 CRUD tools:

```
list_<table>    — list rows (select, eq, order, limit, offset)
get_<table>     — get by primary key
create_<table>  — insert
update_<table>  — update by primary key
delete_<table>  — delete by primary key
```

Each function gets `call_<function>`. Served at `GET /mcp/v1/tools`.

### Common mistakes to avoid

- **Never** import from `@flint/react/dist/...` — always use named exports from `@flint/react`
- **Never** use `primitive_type` as the JSX component name — use the exported React component (e.g. `<DataGrid>` not `<data-grid>`)
- **Never** pass raw SQL to MCP tools — use structured `eq` filters: `{ "status": "shipped" }`
- **Never** hardcode `catalogId` without checking `/a2ui/v1/catalog/` for the live version
- For Flutter, `FlintA2uiTransport` is pure SSE — do NOT use WebSocket
- HTMX routes require a JWT `Authorization: Bearer <token>` header (behind `require_rls`)

---

## Installation

### Claude Code (plugin marketplace)

```bash
claude plugin install flint-ui@prometheus-ags/flint-forge
```

### Claude Code (local checkout)

```bash
# From the root of a local flint-forge clone:
claude plugin install ./skills/flint-ui
```

### OpenCode

Add to your `opencode.json`:

```json
{
  "skills": [
    { "path": "./skills/flint-ui" }
  ]
}
```

### Manual (copy skill files)

Copy `skills/flint-ui/` into your project's skill directory (e.g. `.claude/skills/flint-ui/`).
The skill requires no build step — all catalogs and examples are plain Markdown and TypeScript.

### Verify installation

After installing, the skill reference is available in any session. Test with:

```
What Flint components are available in the feedback category?
```

Expected: Claude lists `alert`, `toast`, `modal`, `dialog`, `loading-spinner`, `progress-bar`, `empty-state`, `error-boundary` with their prop signatures from `catalogs/components.md`.

### Slug accuracy gate (CI)

To verify the skill catalog stays in sync with the live database:

```bash
DATABASE_URL=postgres://... cargo test -p fdb-gateway skill_catalog
```

This test (`crates/fdb-gateway/tests/skill_catalog_test.rs`) compares every slug in
`catalogs/components.md` against `SELECT slug FROM flint_a2ui.components WHERE is_base = true`.
It skips cleanly when `DATABASE_URL` is not set.
