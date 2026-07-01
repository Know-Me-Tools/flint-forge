# p5-c015 — Claude Code Skill Package for Flint Components

**Phase:** 5 — Flint A2UI Component Registry  
**Priority:** P2 (enables Claude Code + agent harnesses to know about Flint components natively)  
**Depends on:** p5-c010 (React SDK — skill embeds React API docs), p5-c013 (OpenDesign integration — SKILL.md format)  
**Blocks:** nothing

---

## What this change delivers

A Claude Code skill package (`SKILL.md` format) that can be installed into any Claude Code session via `claude plugin marketplace add`. The skill gives Claude Code and agent harnesses (Antigravity, Cursor, Windsurf, Zed) native knowledge of:
- All 63 Flint A2UI component slugs and their prop schemas
- The `@flint/react`, `flint_genui` (Flutter), and HTMX renderer APIs
- The `POST /a2ui/v1/surfaces/assemble` and REST API endpoints
- Design token format (W3C 2024) and DESIGN.md import flows

This enables Claude Code to generate Flint UI code correctly without hallucinating component APIs.

---

## Research Basis

See `.firecrawl/opendesign-claude-design-2026.md`. Key patterns:
- Claude Code skills use `SKILL.md` with YAML frontmatter
- Distribution: `claude plugin marketplace add <github-owner/repo>`
- The Skills API accepts `container.skills` parameter on Messages API calls
- Design skills like `ancoleman/ai-design-components` (380 stars) demonstrate the pattern: 76 production-ready skills for UI domains

---

## Skill Package Structure

```
skills/flint-ui/
├── SKILL.md                    # Main skill definition (Claude Code format)
├── catalogs/
│   ├── components.md           # All 63 component slugs with descriptions + prop signatures
│   ├── react-api.md            # @flint/react complete API reference
│   ├── flutter-api.md          # flint_genui complete API reference
│   └── htmx-api.md             # HTMX renderer route reference
├── examples/
│   ├── react-data-grid.tsx     # Example: DataGrid with real Flint props
│   ├── react-agent-chat.tsx    # Example: AgentChat surface
│   ├── flutter-surface.dart    # Example: FlintSurface in Flutter
│   └── htmx-form.html          # Example: Form fragment
└── schemas/
    ├── a2ui-message.json       # A2UI message JSON schema (createSurface, updateComponents, etc.)
    └── design-token.json       # W3C 2024 design token schema
```

### SKILL.md

```yaml
---
name: flint-ui
description: Flint A2UI component library — React, Flutter, HTMX renderers
version: 0.1.0
license: Apache-2.0
allowed-tools:
  - Read
  - Write
  - Edit
  - Bash
metadata:
  category: ui-components
  platform: web, mobile, server
  scenario: component-generation, design-system, agent-ui
  design_system.requires: flint_a2ui
od:
  mode: utility
  fidelity: production
  preview.type: none
---

# Flint UI Skill

You have access to the Flint A2UI component library — a registry of 63 production-grade UI components
for React 19, Flutter/Dart (via flint_genui + genui ^0.9.2), and HTMX (via Axum + Askama).

## When to use this skill

Use this skill when:
- Generating React code that uses `@flint/react` components
- Generating Flutter/Dart code that uses `flint_genui` and `genui` components
- Generating HTMX fragments rendered from fdb-gateway Askama templates
- Assembling A2UI surfaces via `POST /a2ui/v1/surfaces/assemble`
- Designing component overrides using DESIGN.md or W3C design tokens
- Integrating with OpenDesign (`nexu-io/open-design`) or Claude Design

## Component Catalog

See `catalogs/components.md` for the full list of 63 components organized by category:
- Layout (9): Stack, Card, Grid, Split, Tabs, Accordion, Scroll, Modal, Drawer
- Data Display (12): DataGrid, Table, Chart, Timeline, Kanban, Calendar, Metric, Badge, Avatar, Progress, StatusIndicator, RichText
- Input (14+): Form, TextField, TextArea, Number, Select, MultiSelect, DatePicker, FileUpload, Search, JsonEditor, RichEditor, Switch, Checkbox, Radio, FieldArray
- Action (9): Button, IconButton, ButtonGroup, ActionMenu, Confirm, BulkAction, Wizard, ActionBar, QuickAction
- Agent (12): AgentChat, AgentThought, ToolCall, ToolResult, SkillCard, Artifact, StreamingText, StreamingCode, Decision, ProgressLog, Comparison, Suggestion
- Navigation (7): Breadcrumb, NavMenu, CommandPalette, PageHeader, Stepper, Pagination, FilterBar

## React 19 Usage

ALWAYS use `@flint/react` (never `@copilotkit/*` or `@assistant-ui/react`):

```tsx
import { FlintProvider, FlintSurface, registerFlintComponent } from '@flint/react';

// Wrap your app
<FlintProvider endpoint={API_URL} applicationId={APP_ID} jwt={jwt}>
  <FlintSurface surfaceId="main" />
</FlintProvider>

// Override a component
<FlintProvider components={{ DataGrid: MyCustomGrid }}>

// Register a custom component
registerFlintComponent({
  slug: 'my-component',  // Must exist in flint_a2ui.components
  propsSchema: z.object({ ... }),
  component: (props) => <MyWidget {...props} />,
});
```

## Flutter Usage

ALWAYS use `flint_genui` (never raw `genui` without Flint catalog):

```dart
// pubspec.yaml
dependencies:
  flint_genui: ^0.1.0  # includes genui ^0.9.2 and cue ^0.3.11

// Usage
FlintSurface(
  endpoint: 'https://api.myapp.com',
  applicationId: 'app-uuid',
  jwt: userJwt,
)
```

## HTMX Usage

For server-rendered prototypes, use fdb-gateway routes:

```html
<!-- Full page render -->
GET /htmx/components/data-grid

<!-- HTMX fragment -->
POST /htmx/components/form
Content-Type: application/json
{ "table": "public.orders" }

<!-- SSE streaming -->
<div hx-ext="sse" sse-connect="/htmx/stream/{{ surface_id }}">
```

## Design Tokens

Use W3C 2024 format for `flint_a2ui.design_systems.tokens`:
```json
{
  "color": { "primary": { "$value": "#...", "$type": "color" } }
}
```

Import DESIGN.md: `POST /a2ui/v1/design-systems/import` with `{ "format": "design-md", "content": "..." }`

## A2UI Messages

All surfaces receive A2UI protocol messages:
- `createSurface` — create a new surface (renders component tree)
- `updateComponents` — update one or more component props in-place
- `updateDataModel` — push new data to a bound data source
- `deleteSurface` — remove the surface

See `schemas/a2ui-message.json` for the full schema.
```

---

## OpenDesign Plugin Distribution

The same skill package is distributed as an OpenDesign plugin by adding `open-design.json`:

```json
{
  "name": "flint-components",
  "version": "0.1.0", 
  "description": "Flint A2UI component catalog for OpenDesign ideation",
  "skills": ["skills/flint-ui/SKILL.md"],
  "capabilities": ["component-catalog", "design-system-import", "surface-preview"]
}
```

Install in OpenDesign:
```
od plugin install github.com/prometheus-ags/flint-forge/plugins/flint-components
```

Install in Claude Code:
```
claude plugin marketplace add prometheus-ags/flint-forge
claude plugin install flint-ui@prometheus-ags/flint-forge
```

---

## Gate Tests

- [ ] `SKILL.md` validates against Claude Code skill schema (frontmatter fields, allowed-tools)
- [ ] `catalogs/components.md` lists all 63 components with correct slugs matching `flint_a2ui.components`
- [ ] `react-api.md` matches actual `@flint/react` public API surface
- [ ] `flutter-api.md` matches actual `flint_genui` public API surface
- [ ] Skill can be loaded into Claude Code: `claude plugin install flint-ui@...`
- [ ] After skill load: Claude Code generates correct `@flint/react` code (no hallucinated props)
- [ ] After skill load: Claude Code generates correct `flint_genui` Flutter code
- [ ] `open-design.json` validates against OpenDesign plugin manifest schema
