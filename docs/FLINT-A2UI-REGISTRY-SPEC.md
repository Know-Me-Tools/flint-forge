# Flint Global A2UI Component Registry: Functional Specification, Architecture, and Implementation Plan

**Document ID:** RFC-FORGE-A2UI-001  
**Date:** June 2026  
**Status:** Architecture Design — Ready for Implementation  
**Scope:** Global A2UI/AG-UI component registry with metadata-driven extensibility, vector search, design system integration, and real-time dynamic component construction for the entire Prometheus Flint ecosystem.

---

## 1. Executive Summary

The Prometheus Flint platform needs a single, global A2UI component registry that serves as the canonical source of truth for all UI components across the ecosystem — from administrative dashboards in `flint-platform-agent` to dynamically generated interfaces in downstream applications. This registry is not a static component library; it is a **living, metadata-driven, AI-searchable component runtime** that enables agents to discover, compose, and construct interfaces on-the-fly.

### Why This Matters

Modern AI-native platforms require interfaces that are generated at runtime by agents, not hand-coded by developers months in advance. The industry has converged on two paradigms:

- **Constrained generation** (A2UI, Vercel AI SDK, Tambo): agents select from pre-registered components, ensuring safety and consistency.
- **Unconstrained generation** (Claude Artifacts, MCP Apps): agents generate raw HTML/CSS/JS, requiring sandboxing and losing design consistency.

Flint adopts **constrained generation as the default, with unconstrained as an escape hatch** — but the constrained model requires a world-class component registry that agents can query semantically, compose dynamically, and render across platforms.

### What This Document Specifies

1. **Base A2UI component primitives** — the foundational UI vocabulary for administration, function interfaces, and application development.
2. **A metadata-driven, extensible registry schema** — using PostgreSQL JSONB columns for flexible schemas and embeddings tables for semantic search.
3. **Database schema integration** — binding components to tables, functions, and views so dynamic forms and dashboards construct automatically from database metadata.
4. **Application metadata model** — defining applications, permissions, roles, and ownership with JWT-resolvable access control.
5. **Design system integration** — bridging to Open Design's ODSF format for external design system ingestion and token management.
6. **Real-time dynamic construction** — event-driven component assembly based on agent operations, inference results, tool calls, and skill activations.
7. **Implementation roadmap** — phased delivery from core registry to full ecosystem integration.

### Key Differentiators

| Capability | Traditional Component Library | Flint Global A2UI Registry |
|-----------|------------------------------|---------------------------|
| Component storage | Static files, versioned packages | **Database + JSONB + embeddings** |
| Discovery | Keyword search, documentation | **Semantic vector search + hybrid BM25** |
| Schema binding | Manual prop definitions | **Auto-derived from database metadata** |
| Design system | Hardcoded, one per app | **Dynamic, ODSF-compatible, token-injected** |
| Agent construction | Developer-coded pages | **Event-driven, runtime assembly** |
| Cross-platform | Framework-specific | **A2UI-native, framework-agnostic** |
| Security | Code review | **Pre-approved catalog + JWT-scoped access** |
| Real-time updates | Build + deploy | **LISTEN/NOTIFY + Iggy spine** |

---

## 2. Philosophy and Design Principles

### 2.1 The Core Philosophy: Metadata as the Single Source of Truth

Every component, every application, every design token, and every permission is stored in PostgreSQL as **queryable, versioned, auditable metadata**. The registry is not a sidecar service; it is part of the sovereign data layer. Agents query the registry through:

- **SQL** (direct, for precise lookups)
- **REST** (for client applications)
- **A2A tasks** (for agent-to-agent delegation)
- **MCP tools** (for LLM tool calling)
- **Semantic search** (for natural language discovery)

### 2.2 Design Principles

| Principle | Rationale |
|-----------|-----------|
| **Constrained by default** | Agents select from pre-approved components. Raw HTML/JS is sandboxed in iframes with `sandbox="allow-scripts"` only when explicitly enabled. |
| **JSONB for extensibility** | Component schemas, prop definitions, and design tokens are stored in JSONB columns so the schema evolves without migrations. |
| **Embeddings for discovery** | Every component description, prop name, and usage example is embedded so agents find the right component from natural language. |
| **Database-native binding** | Components auto-bind to `flint_meta.cache_tables` and `cache_functions` so a database schema change propagates to UI generation. |
| **Token-aware rendering** | Design tokens are resolved at query time based on the application, tenant, and user preferences. |
| **Identity-propagated** | JWT claims from `flint-gate` flow through to component resolution: which components a user sees depends on their roles and permissions. |
| **Event-driven assembly** | Component trees are constructed reactively in response to agent events: tool calls, inference completions, skill activations, state changes. |
| **Open Design bridge** | ODSF bundles are first-class citizens; Open Design's `DESIGN.md` contracts can be imported, versioned, and referenced. |
| **Application-scoped** | Every component lives in an application context. Base applications (admin, playground) share the same registry as user-defined applications. |
| **Federation-ready** | Component definitions can synchronize across nodes via CRDT, enabling distributed teams to share component libraries. |

### 2.3 The A2UI Primitives Model

A2UI (Google's Agent-to-User Interface protocol) defines a **declarative JSON format** for agent-generated UIs. The Flint registry extends this model with:

- **Database-aware components** that auto-generate props from table/function metadata
- **Permission-scoped components** that filter based on JWT claims
- **Design-token-injected components** that resolve tokens at render time
- **Event-reactive components** that reassemble in response to real-time events

---

## 3. Architecture Overview

### 3.1 The Registry in the Flint Ecosystem

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         FLINT ECOSYSTEM                                      │
│                                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────────────────┐ │
│  │  Flint Gate  │───→│  Flint Forge │───→│   Flint Realtime Fabric     │ │
│  │  (Axum/Rust) │JWT │  (Meta +    │    │   (Iggy / WebSocket / SSE)  │ │
│  │  Kratos/Keto │    │  Reflection)│    │                             │ │
│  │  Cedar/Vault │    │             │    │                             │ │
│  └──────────────┘    └──────┬──────┘    └──────────────────────────────┘ │
│                               │                                             │
│                    ┌──────────┴──────────┐                                  │
│                    │   PostgreSQL 18      │                                  │
│                    │                      │                                  │
│                    │  ┌────────────────┐ │ ←── THIS DOCUMENT              │
│                    │  │  A2UI Registry │ │                                  │
│                    │  │  (flint_a2ui)  │ │                                  │
│                    │  ├────────────────┤ │                                  │
│                    │  │  components    │ │  ← Component definitions       │
│                    │  │  applications  │ │  ← App metadata + permissions  │
│                    │  │  design_systems  │ │  ← ODSF bundles + tokens       │
│                    │  │  embeddings    │ │  ← Vector search index           │
│                    │  │  schemas       │ │  ← JSON Schema + UI hints        │
│                    │  │  bindings      │ │  ← DB table/function linkage     │
│                    │  │  events        │ │  ← Event-driven assembly log     │
│                    │  └────────────────┘ │                                  │
│                    │                      │                                  │
│                    │  flint_meta          │  ← Database metadata (prev doc) │
│                    │  flint_auth          │  ← Auth + RLS                   │
│                    │  flint_vault         │  ← Encryption                   │
│                    └──────────────────────┘                                  │
│                                                                             │
│  External:                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────────────────────┐  │
│  │ Open Design  │  │  ODSF Bundles │  │  Agent Harnesses (Claude,     │  │
│  │ (reference/  │←─┤  (design.md +  │  │  Kimi, Cursor, OpenCode)      │  │
│  │  open-design)│  │   tokens.css)  │  │  via A2A / MCP / A2UI          │  │
│  └──────────────┘  └──────────────┘  └─────────────────────────────────┘  │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │  Flint Platform Agent (fpa)                                         │  │
│  │  Axum server → A2A / A2UI / AG-UI / MCP surfaces                    │  │
│  │  AdminTask → Registry CRUD + Query + Deploy                        │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 The Registry Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    A2UI REGISTRY — INTERNAL ARCHITECTURE                      │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  LAYER 1: DATABASE SCHEMA (flint_a2ui)                              │   │
│  │                                                                      │   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ │   │
│  │  │components│ │applications│ │design_sys│ │embeddings│ │  events  │ │   │
│  │  │  (JSONB) │ │  (JSONB)   │ │  (JSONB) │ │(vector)  │ │  (JSONB) │ │   │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘ │   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐             │   │
│  │  │ schemas  │ │bindings  │ │roles     │ │permissions│             │   │
│  │  │ (JSONB)  │ │(JSONB)   │ │(JSONB)   │ │ (JSONB)   │             │   │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘             │   │
│  │                                                                      │   │
│  │  Event Triggers: DDL → auto-update bindings → NOTIFY 'a2ui_change'  │   │
│  │                                                                      │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│  ┌─────────────────────────────────┴─────────────────────────────────────┐│
│  │  LAYER 2: RUST REFLECTION ENGINE (flint-reflection extension)         ││
│  │                                                                        ││
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────┐  ││
│  │  │  ArcSwap IR  │  │  REST Router │  │  A2A Task    │  │ MCP Tool │  ││
│  │  │  (compiled)  │  │  (Axum)      │  │  Registry    │  │  Server  │  ││
│  │  └──────────────┘  └──────────────┘  └──────────────┘  └──────────┘  ││
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              ││
│  │  │  Vector      │  │  Semantic    │  │  Design      │                ││
│  │  │  Search      │  │  Query       │  │  Token       │                ││
│  │  │  (pgvector)  │  │  Engine      │  │  Resolver    │                ││
│  │  └──────────────┘  └──────────────┘  └──────────────┘              ││
│  │                                                                        ││
│  └───────────────────────────────────────────────────────────────────────┘│
│                                    │                                        │
│  ┌─────────────────────────────────┴─────────────────────────────────────┐│
│  │  LAYER 3: PROTOCOL SURFACES                                          ││
│  │                                                                        ││
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐ ││
│  │  │ REST API │  │ A2A Task │  │ A2UI     │  │ AG-UI    │  │ MCP    │ ││
│  │  │ (HTTP)   │  │ (HTTP)   │  │ (JSON)   │  │ (SSE)    │  │ (HTTP) │ ││
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘  └────────┘ ││
│  │                                                                        ││
│  └───────────────────────────────────────────────────────────────────────┘│
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.3 Component Lifecycle

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      COMPONENT LIFECYCLE                                     │
│                                                                             │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐   │
│  │  Design     │   │  Register   │   │  Discover   │   │  Compose    │   │
│  │  (Open      │──→│  (flint_a2ui│──→│  (Semantic  │──→│  (Agent     │   │
│  │   Design /   │   │   INSERT)   │   │   Search)   │   │   Assembly) │   │
│  │   Manual)    │   │             │   │             │   │             │   │
│  └─────────────┘   └─────────────┘   └─────────────┘   └─────────────┘   │
│                                                             │              │
│                                                             ▼              │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐   │
│  │  Event      │   │  Realtime   │   │  Render     │   │  Feedback   │   │
│  │  Trigger    │←──│  Push       │←──│  (Client)   │←──│  (User      │   │
│  │  (Reassemble)│   │  (Iggy)     │   │             │   │   Action)   │   │
│  └─────────────┘   └─────────────┘   └─────────────┘   └─────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Base A2UI Component Primitives

### 4.1 The Primitive Taxonomy

A2UI defines a small set of core primitives. Flint extends this with **database-aware** and **function-aware** variants that auto-generate from `flint_meta` metadata.

#### 4.1.1 Layout Primitives

| Primitive | A2UI Type | Purpose | Database Binding |
|-----------|-----------|---------|-----------------|
| **Stack** | `stack` | Vertical or horizontal grouping of children | `table` (rows as stack items) |
| **Card** | `card` | Bordered container with header, body, footer | `table` (single row as card) |
| **Grid** | `grid` | Tabular layout with configurable columns | `table` (columns → grid cells) |
| **Split** | `split` | Resizable panes (sidebar + main) | `view` (related tables) |
| **Tabs** | `tabs` | Tabbed interface with lazy loading | `enum` (tab labels from enum values) |
| **Accordion** | `accordion` | Collapsible sections | `table` (grouped by category column) |
| **Scroll** | `scroll` | Scrollable container with virtualized lists | `table` (large result sets) |
| **Modal** | `modal` | Overlay dialog with focus trap | `function` (confirmation actions) |
| **Drawer** | `drawer` | Slide-out panel from edge | `table` (detail view) |

#### 4.1.2 Data Display Primitives

| Primitive | A2UI Type | Purpose | Database Binding |
|-----------|-----------|---------|-----------------|
| **Text** | `text` | Static text, headings, paragraphs | `column` (text/varchar) |
| **RichText** | `rich_text` | Markdown-rendered content | `column` (text with markdown) |
| **Code** | `code` | Syntax-highlighted code block | `column` (text, detected language) |
| **Table** | `table` | Tabular data with sorting, filtering | `table` (full row display) |
| **DataGrid** | `data_grid` | Advanced table with pagination, search | `table` + `view` (complex queries) |
| **List** | `list` | Ordered/unordered list | `column` (array type) |
| **Tree** | `tree` | Hierarchical tree view | `table` (self-referencing FK) |
| **Timeline** | `timeline` | Time-ordered events | `table` (timestamp column) |
| **Kanban** | `kanban` | Drag-and-drop status board | `table` (status column + ordering) |
| **Calendar** | `calendar` | Date/time grid with events | `table` (timestamp range columns) |
| **Chart** | `chart` | Data visualization (bar, line, pie) | `table` + `function` (aggregated queries) |
| **Metric** | `metric` | Single number with trend indicator | `function` (scalar return) |
| **Badge** | `badge` | Status indicator with color | `column` (enum/status type) |
| **Avatar** | `avatar` | User/image representation | `column` (uuid referencing users) |
| **Progress** | `progress` | Progress bar or step indicator | `column` (numeric percentage) |
| **StatusIndicator** | `status_indicator` | Colored dot with tooltip | `column` (boolean/enum) |

#### 4.1.3 Input Primitives

| Primitive | A2UI Type | Purpose | Database Binding |
|-----------|-----------|---------|-----------------|
| **TextField** | `text_field` | Single-line text input | `column` (varchar/text) |
| **TextArea** | `text_area` | Multi-line text input | `column` (text) |
| **Number** | `number` | Numeric input with validation | `column` (int/float/numeric) |
| **Slider** | `slider` | Range selection via drag | `column` (numeric range) |
| **Switch** | `switch` | Boolean toggle | `column` (boolean) |
| **Checkbox** | `checkbox` | Multiple selection | `column` (boolean/array) |
| **Radio** | `radio` | Single selection from options | `column` (enum) |
| **Select** | `select` | Dropdown selection | `column` (FK reference → options) |
| **MultiSelect** | `multi_select` | Multiple dropdown selection | `column` (array/FK array) |
| **DatePicker** | `date_picker` | Date selection | `column` (date/timestamp) |
| **DateRange** | `date_range` | Date range selection | `column` (daterange/tsrange) |
| **FileUpload** | `file_upload` | File upload with preview | `column` (bytea/text URL) |
| **ImageUpload** | `image_upload` | Image upload with crop | `column` (bytea/text URL) |
| **Search** | `search` | Search with autocomplete | `function` (fuzzy search function) |
| **RichEditor** | `rich_editor` | WYSIWYG rich text editor | `column` (text with HTML) |
| **JsonEditor** | `json_editor` | JSON editing with validation | `column` (jsonb) |
| **Form** | `form` | Grouped input validation | `table` (multi-column form) |
| **FormSection** | `form_section` | Collapsible form grouping | `table` (column groups) |
| **FieldArray** | `field_array` | Repeating field groups | `column` (array of composite) |

#### 4.1.4 Action Primitives

| Primitive | A2UI Type | Purpose | Database Binding |
|-----------|-----------|---------|-----------------|
| **Button** | `button` | Clickable action trigger | `function` (parameterless RPC) |
| **IconButton** | `icon_button` | Compact action with icon | `function` (parameterless RPC) |
| **ButtonGroup** | `button_group` | Mutually exclusive actions | `enum` (action selection) |
| **ActionMenu** | `action_menu` | Dropdown action list | `function` (multiple RPCs) |
| **Confirm** | `confirm` | Confirmation dialog | `function` (destructive RPC) |
| **BulkAction** | `bulk_action` | Action on selected rows | `table` + `function` (multi-row RPC) |
| **QuickAction** | `quick_action` | Inline action in table/card | `function` (single-row RPC) |
| **Wizard** | `wizard` | Multi-step guided action | `function` (multi-step RPC) |
| **ActionBar** | `action_bar` | Top-level action container | `table` (CRUD operations) |

#### 4.1.5 Agent-Specific Primitives

| Primitive | A2UI Type | Purpose | Source |
|-----------|-----------|---------|--------|
| **AgentChat** | `agent_chat` | Chat interface with streaming | Agent inference events |
| **AgentThought** | `agent_thought` | Chain-of-thought display | Agent reasoning steps |
| **ToolCall** | `tool_call` | Tool invocation card with progress | A2A/MCP tool call events |
| **ToolResult** | `tool_result` | Tool output display (JSON, table, etc.) | Tool return value |
| **SkillCard** | `skill_card` | Skill activation with parameters | Skill registry metadata |
| **Artifact** | `artifact` | Generated artifact (code, image, doc) | Agent output |
| **StreamingText** | `streaming_text` | Live text generation display | AG-UI text delta events |
| **StreamingCode** | `streaming_code` | Live code generation with syntax | AG-UI code delta events |
| **Decision** | `decision` | Agent asking user to choose | Agent input_required state |
| **ProgressLog** | `progress_log` | Real-time operation log | Agent working state |
| **Comparison** | `comparison` | Side-by-side diff view | Agent before/after output |
| **Suggestion** | `suggestion` | Inline suggestion chips | Agent completion hints |

#### 4.1.6 Navigation Primitives

| Primitive | A2UI Type | Purpose | Database Binding |
|-----------|-----------|---------|-----------------|
| **Breadcrumb** | `breadcrumb` | Hierarchical navigation | `table` (parent FK chain) |
| **NavMenu** | `nav_menu` | Sidebar/top navigation | `table` (menu item table) |
| **CommandPalette** | `command_palette` | Quick command search | `function` (command registry) |
| **PageHeader** | `page_header` | Page title with actions | `table` (metadata) |
| **Stepper** | `stepper` | Multi-step progress indicator | `table` (workflow state) |
| **Pagination** | `pagination` | Page navigation for tables | `table` (offset/limit) |
| **FilterBar** | `filter_bar` | Active filter display + removal | `table` (filter state) |

### 4.2 The Component Definition Schema

Every component is stored as a JSONB document in `flint_a2ui.components`. The schema is versioned and extensible:

```json
{
  "id": "comp-uuid",
  "slug": "data-grid",
  "name": "Data Grid",
  "description": "Advanced tabular display with sorting, filtering, pagination, and row actions",
  "long_description": "A production-grade data table component that supports...",
  "category": "data-display",
  "primitive_type": "data_grid",
  "version": "1.2.0",
  "status": "stable",
  
  "schema": {
    "type": "object",
    "properties": {
      "data_source": {
        "type": "string",
        "description": "Table or view name",
        "x-binding": "table",
        "x-table-schema": "public",
        "x-table-name": "customers"
      },
      "columns": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "field": { "type": "string", "x-binding": "column" },
            "header": { "type": "string" },
            "sortable": { "type": "boolean", "default": true },
            "filterable": { "type": "boolean", "default": true },
            "width": { "type": "string", "default": "auto" },
            "component": { "type": "string", "x-ref": "component-slug" }
          }
        }
      },
      "pagination": {
        "type": "object",
        "properties": {
          "page_size": { "type": "integer", "default": 25 },
          "page_size_options": { "type": "array", "default": [10, 25, 50, 100] }
        }
      },
      "row_actions": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "label": { "type": "string" },
            "icon": { "type": "string" },
            "action": { "type": "string", "x-binding": "function" },
            "requires_selection": { "type": "boolean", "default": false }
          }
        }
      },
      "bulk_actions": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "label": { "type": "string" },
            "action": { "type": "string", "x-binding": "function" }
          }
        }
      }
    },
    "required": ["data_source"]
  },
  
  "ui_hints": {
    "min_width": "400px",
    "max_width": "100%",
    "min_height": "200px",
    "responsive": true,
    "mobile_variant": "data-grid-mobile",
    "theme_mode": "auto"
  },
  
  "platforms": {
    "web": { "frameworks": ["react", "vue", "svelte", "lit"] },
    "desktop": { "frameworks": ["tauri", "electron"] },
    "mobile": { "frameworks": ["flutter", "react-native"] },
    "cli": { "frameworks": ["ratatui"] }
  },
  
  "implementations": {
    "react": {
      "package": "@flint/a2ui-react",
      "import": "DataGrid",
      "props_map": { "data_source": "rows", "columns": "columns" }
    },
    "vue": {
      "package": "@flint/a2ui-vue",
      "import": "FlintDataGrid"
    }
  },
  
  "design_tokens": {
    "colors": ["surface", "surface-elevated", "accent", "text-primary"],
    "typography": ["font-mono", "font-body"],
    "spacing": ["space-md", "space-lg"]
  },
  
  "accessibility": {
    "aria_role": "grid",
    "keyboard_navigable": true,
    "screen_reader_optimized": true,
    "wcag_level": "AA"
  },
  
  "examples": [
    {
      "title": "Customer List",
      "config": { "data_source": "public.customers", "columns": [...] }
    }
  ],
  
  "related_components": ["table", "card", "pagination", "filter-bar"],
  
  "permissions": {
    "read_roles": ["admin", "operator", "viewer"],
    "write_roles": ["admin", "operator"]
  }
}
```

### 4.3 Database-Binding Extensions (`x-*` properties)

The JSON Schema uses `x-*` extensions to bind to database metadata:

| Extension | Target | Description |
|-----------|--------|-------------|
| `x-binding` | `table`, `column`, `function`, `view` | What database object this prop binds to |
| `x-table-schema` | schema name | The schema containing the table |
| `x-table-name` | table name | The table or view to bind |
| `x-column-name` | column name | The specific column |
| `x-function-schema` | schema name | Schema containing the function |
| `x-function-name` | function name | The function to call |
| `x-ref` | component slug | Reference to another component definition |
| `x-foreign-key` | `table.column` | FK relationship for select/autocomplete |
| `x-enum-values` | `[]` | Enum values from `flint_meta.cache_types` |
| `x-rls-policy` | policy name | Which RLS policy applies to this binding |
| `x-encrypted` | boolean | Whether the column uses Vault encryption |
| `x-keto-namespace` | namespace | Keto namespace for permission checks |
| `x-keto-relation` | relation | Keto relation for this action |

---

## 5. Registry Schema: Metadata-Driven, Extensible Storage

### 5.1 Core Tables

#### 5.1.1 `flint_a2ui.components` — Component Definitions

```sql
CREATE TABLE flint_a2ui.components (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    slug            text NOT NULL UNIQUE,
    name            text NOT NULL,
    description     text NOT NULL,
    long_description text,
    
    -- Taxonomy
    category        text NOT NULL CHECK (category IN (
        'layout', 'data-display', 'input', 'action', 'agent', 'navigation',
        'feedback', 'media', 'chart', 'custom'
    )),
    primitive_type  text NOT NULL,  -- maps to A2UI primitive type
    
    -- Versioning
    version         text NOT NULL DEFAULT '1.0.0',
    status          text NOT NULL DEFAULT 'draft' CHECK (status IN (
        'draft', 'experimental', 'stable', 'deprecated', 'archived'
    )),
    
    -- Schema (JSONB - extensible)
    schema          jsonb NOT NULL DEFAULT '{}',
    ui_hints        jsonb NOT NULL DEFAULT '{}',
    platforms       jsonb NOT NULL DEFAULT '{}',
    implementations jsonb NOT NULL DEFAULT '{}',
    design_tokens   jsonb NOT NULL DEFAULT '{}',
    accessibility   jsonb NOT NULL DEFAULT '{}',
    examples        jsonb NOT NULL DEFAULT '[]',
    related_components jsonb NOT NULL DEFAULT '[]',
    
    -- Permissions (can be overridden by application context)
    permissions     jsonb NOT NULL DEFAULT '{}',
    
    -- Application context
    application_id  uuid REFERENCES flint_a2ui.applications(id),
    is_base         boolean NOT NULL DEFAULT false,  -- true = available to all apps
    
    -- Metadata
    created_by      uuid NOT NULL,  -- user/agent ID
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    
    -- Search
    search_vector   tsvector,
    
    -- Soft delete
    deleted_at      timestamptz,
    
    CONSTRAINT valid_semver CHECK (version ~ '^\d+\.\d+\.\d+(-[a-zA-Z0-9.-]+)?$')
);

CREATE INDEX idx_components_category ON flint_a2ui.components(category);
CREATE INDEX idx_components_primitive ON flint_a2ui.components(primitive_type);
CREATE INDEX idx_components_status ON flint_a2ui.components(status);
CREATE INDEX idx_components_app ON flint_a2ui.components(application_id) WHERE application_id IS NOT NULL;
CREATE INDEX idx_components_is_base ON flint_a2ui.components(is_base) WHERE is_base = true;
CREATE INDEX idx_components_search ON flint_a2ui.components USING GIN(search_vector);
CREATE INDEX idx_components_schema ON flint_a2ui.components USING GIN(schema jsonb_path_ops);
```

#### 5.1.2 `flint_a2ui.applications` — Application Metadata

```sql
CREATE TABLE flint_a2ui.applications (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    slug            text NOT NULL UNIQUE,
    name            text NOT NULL,
    description     text,
    
    -- Application type
    app_type        text NOT NULL CHECK (app_type IN (
        'system',       -- admin, playground, monitoring
        'platform',     -- flint-platform-agent, flint-gate
        'user',         -- user-created application
        'template',     -- template for new apps
        'integration'   -- external service integration
    )),
    
    -- Ownership
    owner_id        uuid NOT NULL,  -- user or organization ID
    owner_type      text NOT NULL CHECK (owner_type IN ('user', 'organization')),
    
    -- Application configuration (JSONB - extensible)
    config          jsonb NOT NULL DEFAULT '{}',
    -- Example config:
    -- {
    --   "design_system_id": "uuid",
    --   "default_route": "/dashboard",
    --   "features": ["realtime", "collaboration", "offline"],
    --   "auth": { "method": "jwt", "provider": "kratos" },
    --   "database": { "schema": "app_customers", "isolate_tenant": true },
    --   "realtime": { "channels": ["app.events", "app.notifications"] }
    -- }
    
    -- Design system binding
    design_system_id uuid REFERENCES flint_a2ui.design_systems(id),
    
    -- Status
    status          text NOT NULL DEFAULT 'active' CHECK (status IN (
        'draft', 'active', 'suspended', 'archived'
    )),
    
    -- JWT claims template (injected into tokens for this app)
    jwt_claims_template jsonb NOT NULL DEFAULT '{}',
    -- Example: { "aud": "app-slug", "roles": ["app-user"], "tenant_id": "..." }
    
    -- Metadata
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    deleted_at      timestamptz
);

CREATE INDEX idx_applications_owner ON flint_a2ui.applications(owner_id, owner_type);
CREATE INDEX idx_applications_type ON flint_a2ui.applications(app_type);
CREATE INDEX idx_applications_status ON flint_a2ui.applications(status);
CREATE INDEX idx_applications_design_system ON flint_a2ui.applications(design_system_id);
```

#### 5.1.3 `flint_a2ui.design_systems` — ODSF-Compatible Design Systems

```sql
CREATE TABLE flint_a2ui.design_systems (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    slug            text NOT NULL UNIQUE,
    name            text NOT NULL,
    description     text,
    
    -- ODSF metadata
    odsf_version    text NOT NULL DEFAULT '0.1',
    source_url      text,  -- URL to ODSF bundle (open-design.ai, etc.)
    source_type     text CHECK (source_type IN ('odsf', 'open-design', 'custom', 'imported')),
    
    -- The DESIGN.md contract content
    design_md       text,  -- Full DESIGN.md content
    
    -- Tokens (extracted from tokens.css or design-tokens.json)
    tokens          jsonb NOT NULL DEFAULT '{}',
    -- Example: {
    --   "colors": { "bg": "#0B0F14", "surface": "#131A22", "accent": "#FF6A3D" },
    --   "typography": { "display": "Space Grotesk", "body": "Inter", "mono": "JetBrains Mono" },
    --   "spacing": { "xs": "4px", "sm": "8px", "md": "16px", "lg": "24px", "xl": "48px" },
    --   "breakpoints": { "mobile": "640px", "tablet": "768px", "desktop": "1024px" }
    -- }
    
    -- Component style mappings (which design tokens each component uses)
    component_tokens jsonb NOT NULL DEFAULT '{}',
    -- Example: { "button": { "bg": "accent", "text": "bg", "border": "surface-2" } }
    
    -- CSS output (pre-generated for each platform)
    css_output      jsonb NOT NULL DEFAULT '{}',
    -- Example: { "web": "/* generated CSS */", "desktop": "/* Tauri CSS */" }
    
    -- Status
    status          text NOT NULL DEFAULT 'active' CHECK (status IN ('draft', 'active', 'deprecated')),
    
    -- Metadata
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now()
);
```

#### 5.1.4 `flint_a2ui.embeddings` — Vector Search Index

```sql
CREATE TABLE flint_a2ui.embeddings (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Polymorphic reference to embedded entity
    entity_type     text NOT NULL CHECK (entity_type IN (
        'component', 'application', 'design_system', 'schema', 'example'
    )),
    entity_id       uuid NOT NULL,
    
    -- What aspect of the entity this embedding represents
    aspect          text NOT NULL CHECK (aspect IN (
        'description',       -- Full description text
        'schema_props',      -- Concatenated prop names + descriptions
        'usage_example',     -- Example usage code/text
        'design_tokens',     -- Token names + descriptions
        'category_tags',     -- Category + tags + primitive type
        'full_text'          -- All text concatenated
    )),
    
    -- The embedding vector (using pgvector)
    embedding       vector(1536),  -- OpenAI text-embedding-3-large
    
    -- Source text (for debugging and re-embedding)
    source_text     text NOT NULL,
    
    -- Model used
    model           text NOT NULL DEFAULT 'text-embedding-3-large',
    model_version   text,
    
    -- Metadata
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    
    -- Unique constraint: one embedding per entity + aspect + model
    UNIQUE(entity_type, entity_id, aspect, model)
);

-- Create HNSW index for fast approximate nearest neighbor search
CREATE INDEX idx_embeddings_hnsw ON flint_a2ui.embeddings 
    USING hnsw (embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);

CREATE INDEX idx_embeddings_entity ON flint_a2ui.embeddings(entity_type, entity_id);
```

**Requires:** `pgvector` extension for PostgreSQL.

#### 5.1.5 `flint_a2ui.schemas` — JSON Schema Registry

```sql
CREATE TABLE flint_a2ui.schemas (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    slug            text NOT NULL UNIQUE,
    name            text NOT NULL,
    description     text,
    
    -- Schema type
    schema_type     text NOT NULL CHECK (schema_type IN (
        'component_props',   -- Component property schema
        'data_model',        -- Data structure schema (table-like)
        'form_schema',       -- Form validation + UI schema combined
        'api_request',       -- API request body schema
        'api_response',      -- API response schema
        'event_payload',     -- Event payload schema
        'design_token',      -- Design token schema
        'ui_schema'          -- UI-only schema (presentation layer)
    )),
    
    -- The JSON Schema document
    schema_json     jsonb NOT NULL,
    
    -- UI schema (presentation hints separate from data schema)
    ui_schema_json  jsonb,
    
    -- Binding to database object
    binding         jsonb,  -- { "type": "table", "schema": "public", "name": "customers" }
    
    -- Versioning
    version         text NOT NULL DEFAULT '1.0.0',
    
    -- Application scope
    application_id  uuid REFERENCES flint_a2ui.applications(id),
    is_base         boolean NOT NULL DEFAULT false,
    
    -- Metadata
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_schemas_type ON flint_a2ui.schemas(schema_type);
CREATE INDEX idx_schemas_app ON flint_a2ui.schemas(application_id);
CREATE INDEX idx_schemas_binding ON flint_a2ui.schemas USING GIN(binding jsonb_path_ops);
```

#### 5.1.6 `flint_a2ui.bindings` — Database-to-Component Linkage

```sql
CREATE TABLE flint_a2ui.bindings (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- What is being bound
    binding_type    text NOT NULL CHECK (binding_type IN (
        'table', 'view', 'function', 'column', 'relationship', 'enum'
    )),
    schema_name     text NOT NULL,
    object_name     text NOT NULL,
    column_name     text,  -- NULL for table/view/function bindings
    
    -- What it binds to
    component_id    uuid REFERENCES flint_a2ui.components(id),
    schema_id       uuid REFERENCES flint_a2ui.schemas(id),
    
    -- Binding configuration (how the DB object maps to component props)
    config          jsonb NOT NULL DEFAULT '{}',
    -- Example for table → data-grid:
    -- {
    --   "prop_mapping": { "data_source": "table", "columns": "auto" },
    --   "column_overrides": { "email": { "component": "email-link" } },
    --   "row_actions": ["edit", "delete"],
    --   "bulk_actions": ["delete-selected"]
    -- }
    
    -- Auto-generated flag (created by reflection, not manual)
    is_auto_generated boolean NOT NULL DEFAULT false,
    
    -- Confidence score (for auto-generated bindings)
    confidence      numeric(3,2) CHECK (confidence >= 0 AND confidence <= 1),
    
    -- Application scope
    application_id  uuid REFERENCES flint_a2ui.applications(id),
    
    -- Metadata
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    
    UNIQUE(binding_type, schema_name, object_name, column_name, application_id)
);

CREATE INDEX idx_bindings_object ON flint_a2ui.bindings(binding_type, schema_name, object_name);
CREATE INDEX idx_bindings_component ON flint_a2ui.bindings(component_id);
CREATE INDEX idx_bindings_app ON flint_a2ui.bindings(application_id);
```

### 5.2 Event-Driven Assembly Log

```sql
CREATE TABLE flint_a2ui.events (
    id              bigserial PRIMARY KEY,
    
    -- Event classification
    event_type      text NOT NULL CHECK (event_type IN (
        'component_registered', 'component_updated', 'component_deprecated',
        'binding_created', 'binding_updated', 'binding_auto_generated',
        'schema_changed', 'design_system_imported', 'design_token_updated',
        'application_created', 'application_config_changed',
        'component_queried', 'component_rendered', 'user_interaction'
    )),
    
    -- Source (what triggered the event)
    source_type     text NOT NULL CHECK (source_type IN (
        'agent', 'user', 'system', 'event_trigger', 'mcp_tool', 'a2a_task'
    )),
    source_id       text,  -- Agent ID, user ID, or system process ID
    
    -- Target (what was affected)
    target_type     text NOT NULL CHECK (target_type IN (
        'component', 'application', 'binding', 'schema', 'design_system', 'surface'
    )),
    target_id       uuid,
    
    -- Event payload
    payload         jsonb NOT NULL DEFAULT '{}',
    
    -- Application context
    application_id  uuid REFERENCES flint_a2ui.applications(id),
    
    -- JWT claims of the actor (for audit)
    actor_claims    jsonb,
    
    -- Timestamp
    created_at      timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_events_type ON flint_a2ui.events(event_type);
CREATE INDEX idx_events_source ON flint_a2ui.events(source_type, source_id);
CREATE INDEX idx_events_target ON flint_a2ui.events(target_type, target_id);
CREATE INDEX idx_events_app ON flint_a2ui.events(application_id);
CREATE INDEX idx_events_created ON flint_a2ui.events(created_at);

-- NOTIFY channel for real-time event streaming
-- flint_a2ui will emit NOTIFY 'a2ui_event', json_build_object(...)
```

---

## 6. Application Metadata Model

### 6.1 The Application Concept

An **Application** in the Flint registry is a self-contained unit of functionality with its own:

- **Component catalog** — components scoped to this application
- **Design system** — visual identity and tokens
- **Database schema** — tables and functions for this app
- **Permissions** — roles and access control
- **JWT claims template** — claims injected into tokens for this app's users
- **Real-time channels** — Iggy channels for this app's events
- **Configuration** — feature flags, routing, auth settings

### 6.2 Base Applications

These are built-in applications that ship with the platform:

| Application | Slug | Type | Purpose |
|-------------|------|------|---------|
| **Flint Admin** | `flint-admin` | `system` | Platform administration dashboard |
| **Flint Playground** | `flint-playground` | `system` | Component testing and exploration |
| **Flint Monitoring** | `flint-monitoring` | `system` | Metrics, logs, health dashboards |
| **Flint Registry Manager** | `flint-registry` | `system` | Component registry management UI |
| **Flint Gate Console** | `flint-gate-console` | `platform` | Auth proxy management |
| **Flint Platform Agent** | `flint-platform-agent` | `platform` | Administrative agent interface |

### 6.3 Application Roles and Permissions

```sql
CREATE TABLE flint_a2ui.roles (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    application_id  uuid NOT NULL REFERENCES flint_a2ui.applications(id),
    slug            text NOT NULL,
    name            text NOT NULL,
    description     text,
    
    -- Role hierarchy (inherit permissions from parent role)
    parent_role_id  uuid REFERENCES flint_a2ui.roles(id),
    
    -- Permissions (JSONB - extensible)
    permissions     jsonb NOT NULL DEFAULT '{}',
    -- Example: {
    --   "components": { "read": ["*"], "write": ["custom-*"] },
    --   "bindings": { "read": ["*"], "write": ["own"] },
    --   "schemas": { "read": ["*"], "write": [] },
    --   "applications": { "read": ["self"], "write": ["self"] },
    --   "design_systems": { "read": ["*"], "write": [] },
    --   "events": { "read": ["own"], "write": [] }
    -- }
    
    -- Keto integration
    keto_namespace  text,
    keto_relation   text,
    
    -- Metadata
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    
    UNIQUE(application_id, slug)
);

CREATE TABLE flint_a2ui.role_assignments (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         uuid NOT NULL,  -- References flint_auth.users (or Kratos identity)
    role_id         uuid NOT NULL REFERENCES flint_a2ui.roles(id),
    application_id  uuid NOT NULL REFERENCES flint_a2ui.applications(id),
    
    -- Context (e.g., tenant-specific role)
    context         jsonb NOT NULL DEFAULT '{}',
    -- Example: { "tenant_id": "uuid", "project_id": "uuid" }
    
    -- Source of assignment (who granted it)
    granted_by      uuid,
    granted_at      timestamptz NOT NULL DEFAULT now(),
    expires_at      timestamptz,
    
    UNIQUE(user_id, role_id, application_id, context)
);
```

### 6.4 JWT Claims Resolution

When a user authenticates through `flint-gate` (Kratos), the JWT claims template from the application is merged with the user's resolved roles:

```json
{
  "sub": "user-uuid",
  "iss": "flint-gate",
  "aud": "my-app-slug",
  "iat": 1719830400,
  "exp": 1719834000,
  
  // Flint-specific claims
  "flint": {
    "user_id": "user-uuid",
    "organization_id": "org-uuid",
    "roles": ["admin", "editor"],
    "permissions": {
      "components": ["read:*", "write:custom-*"],
      "bindings": ["read:*", "write:own"]
    },
    "applications": ["my-app-slug", "flint-admin"],
    "tenant_id": "tenant-uuid",
    "keto_subject": "user:user-uuid",
    "keto_namespace": "my-app",
    "vault_key_id": "key-123"
  }
}
```

The `flint-reflection` engine resolves these claims to filter the component catalog:

```sql
-- SQL function exposed by flint_a2ui extension
SELECT flint_a2ui.resolve_components(
    application_id := 'app-uuid',
    jwt_claims := '{"flint": {"roles": ["editor"], "permissions": {"components": ["read:*"]}}}'::jsonb
);
-- Returns: array of component slugs visible to this user
```

---

## 7. Database Schema Integration

### 7.1 Auto-Binding from flint_meta

When `flint_meta` reflects a new table, the A2UI registry can auto-generate bindings:

```sql
-- Event trigger that fires after flint_meta.cache_tables is updated
CREATE OR REPLACE FUNCTION flint_a2ui.auto_generate_bindings()
RETURNS trigger AS $$
DECLARE
    table_schema text := NEW.schema_name;
    table_name text := NEW.table_name;
    app_id uuid;
    binding_config jsonb;
BEGIN
    -- Find the application that owns this schema
    SELECT id INTO app_id
    FROM flint_a2ui.applications
    WHERE config->>'database_schema' = table_schema;
    
    IF app_id IS NOT NULL THEN
        -- Generate default binding: table → data-grid
        binding_config := jsonb_build_object(
            'prop_mapping', jsonb_build_object(
                'data_source', table_schema || '.' || table_name,
                'columns', 'auto'
            ),
            'row_actions', jsonb_build_array('view', 'edit', 'delete'),
            'auto_generated', true
        );
        
        INSERT INTO flint_a2ui.bindings (
            binding_type, schema_name, object_name,
            component_id, config, is_auto_generated, confidence, application_id
        )
        SELECT 
            'table', table_schema, table_name,
            c.id, binding_config, true, 0.95, app_id
        FROM flint_a2ui.components c
        WHERE c.slug = 'data-grid' AND c.is_base = true
        ON CONFLICT (binding_type, schema_name, object_name, column_name, application_id)
        DO UPDATE SET config = EXCLUDED.config, updated_at = now();
        
        -- Emit event for real-time notification
        PERFORM pg_notify('a2ui_event', json_build_object(
            'event_type', 'binding_auto_generated',
            'target_type', 'binding',
            'target_id', NEW.id,
            'application_id', app_id
        )::text);
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER a2ui_auto_bind_tables
    AFTER INSERT ON flint_meta.cache_tables
    FOR EACH ROW
    EXECUTE FUNCTION flint_a2ui.auto_generate_bindings();
```

### 7.2 Column-to-Component Mapping

```sql
-- Map column types to default component types
CREATE TABLE flint_a2ui.type_component_map (
    id              serial PRIMARY KEY,
    pg_type         text NOT NULL UNIQUE,
    component_slug  text NOT NULL REFERENCES flint_a2ui.components(slug),
    ui_hint         jsonb NOT NULL DEFAULT '{}',
    -- Example: { "format": "email", "validators": ["email"] } for text → email
    priority        int NOT NULL DEFAULT 100
);

-- Default mappings
INSERT INTO flint_a2ui.type_component_map (pg_type, component_slug, ui_hint, priority) VALUES
    ('text', 'text-field', '{}', 100),
    ('varchar', 'text-field', '{}', 100),
    ('uuid', 'text-field', '{"read_only": true, "format": "uuid"}', 100),
    ('integer', 'number', '{"step": 1}', 100),
    ('bigint', 'number', '{"step": 1}', 100),
    ('numeric', 'number', '{"precision": true}', 100),
    ('boolean', 'switch', '{}', 100),
    ('timestamp', 'date-picker', '{"include_time": true}', 100),
    ('timestamptz', 'date-picker', '{"include_time": true, "utc": true}', 100),
    ('date', 'date-picker', '{"include_time": false}', 100),
    ('jsonb', 'json-editor', '{}', 100),
    ('text[]', 'multi-select', '{"creatable": true}', 100),
    ('bytea', 'file-upload', '{"accept": "*/*"}', 100);
```

### 7.3 Function-to-Component Mapping

```sql
-- Map functions to action components
CREATE TABLE flint_a2ui.function_component_map (
    id              serial PRIMARY KEY,
    schema_name     text NOT NULL,
    function_name   text NOT NULL,
    component_slug  text NOT NULL REFERENCES flint_a2ui.components(slug),
    
    -- How function parameters map to component props
    param_mapping   jsonb NOT NULL DEFAULT '{}',
    -- Example: { "arg_1": "value", "arg_2": "target_id" }
    
    -- Return type handling
    return_handling text NOT NULL DEFAULT 'display' CHECK (return_handling IN (
        'display', 'redirect', 'refresh', 'download', 'notify'
    )),
    
    UNIQUE(schema_name, function_name)
);
```

---

## 8. Design System Integration and Open Design Bridge

### 8.1 The Open Design Bridge

The Open Design project (maintained at `/Users/gqadonis/Projects/references/open-design`) uses `DESIGN.md` contracts and ODSF bundles. The Flint registry provides a bridge:

```
Open Design                          Flint A2UI Registry
─────────────────                    ───────────────────
DESIGN.md  ──parse──→  Design System Record (flint_a2ui.design_systems)
                        tokens.css ──extract──→  JSONB tokens
                        
tokens.css  ←──generate──  Token resolver (runtime)
                        
components.html  ──import──→  Component definitions with
                              design token references
                        
open-design.json  ──sync──→  Application metadata
                        (skill registry → component registry)
```

### 8.2 ODSF Import Pipeline

```sql
-- Function to import an ODSF bundle
CREATE OR REPLACE FUNCTION flint_a2ui.import_odsf(
    bundle_url text,
    application_id uuid DEFAULT NULL
)
RETURNS uuid AS $$
DECLARE
    ds_id uuid;
    ds_tokens jsonb;
    ds_css jsonb;
BEGIN
    -- 1. Fetch ODSF bundle (delegated to Rust layer via HTTP)
    -- 2. Parse DESIGN.md for metadata
    -- 3. Extract tokens.css → JSONB
    -- 4. Generate component token mappings
    -- 5. Insert design_systems record
    
    INSERT INTO flint_a2ui.design_systems (
        slug, name, description, source_url, source_type,
        design_md, tokens, component_tokens, css_output
    )
    VALUES (
        'imported-' || md5(bundle_url),
        'Imported from ' || bundle_url,
        'Auto-imported ODSF bundle',
        bundle_url,
        'odsf',
        '(DESIGN.md content)',
        ds_tokens,
        '{}',
        ds_css
    )
    RETURNING id INTO ds_id;
    
    -- 6. Generate embeddings for semantic search
    PERFORM flint_a2ui.generate_embedding('design_system', ds_id, 'description');
    PERFORM flint_a2ui.generate_embedding('design_system', ds_id, 'design_tokens');
    
    -- 7. Emit event
    PERFORM pg_notify('a2ui_event', json_build_object(
        'event_type', 'design_system_imported',
        'target_type', 'design_system',
        'target_id', ds_id
    )::text);
    
    RETURN ds_id;
END;
$$ LANGUAGE plpgsql;
```

### 8.3 Token Resolution at Runtime

```sql
-- Function to resolve design tokens for a given application and component
CREATE OR REPLACE FUNCTION flint_a2ui.resolve_tokens(
    app_id uuid,
    component_slug text,
    user_prefs jsonb DEFAULT '{}'
)
RETURNS jsonb AS $$
DECLARE
    ds_id uuid;
    tokens jsonb;
    component_token_map jsonb;
BEGIN
    -- Get the application's design system
    SELECT design_system_id INTO ds_id
    FROM flint_a2ui.applications
    WHERE id = app_id;
    
    -- Get all tokens from the design system
    SELECT ds.tokens INTO tokens
    FROM flint_a2ui.design_systems ds
    WHERE ds.id = ds_id;
    
    -- Get component-specific token mapping
    SELECT ds.component_tokens->component_slug INTO component_token_map
    FROM flint_a2ui.design_systems ds
    WHERE ds.id = ds_id;
    
    -- Merge: base tokens + component-specific overrides + user preferences
    RETURN tokens || COALESCE(component_token_map, '{}') || COALESCE(user_prefs, '{}');
END;
$$ LANGUAGE plpgsql;
```

### 8.4 Two-Way Sync: Flint → Open Design

Flint can export its component library back to Open Design as an ODSF bundle:

```sql
-- Export component library as ODSF bundle
CREATE OR REPLACE FUNCTION flint_a2ui.export_odsf(
    application_id uuid
)
RETURNS jsonb AS $$
DECLARE
    result jsonb;
BEGIN
    SELECT jsonb_build_object(
        'format', 'odsf',
        'version', '0.1',
        'generated_at', now(),
        'application', (SELECT jsonb_build_object(
            'slug', slug,
            'name', name,
            'description', description
        ) FROM flint_a2ui.applications WHERE id = application_id),
        'components', (
            SELECT jsonb_agg(jsonb_build_object(
                'slug', c.slug,
                'name', c.name,
                'description', c.description,
                'category', c.category,
                'schema', c.schema,
                'examples', c.examples
            ))
            FROM flint_a2ui.components c
            WHERE c.application_id = application_id OR c.is_base = true
        ),
        'tokens', (
            SELECT ds.tokens
            FROM flint_a2ui.design_systems ds
            JOIN flint_a2ui.applications a ON a.design_system_id = ds.id
            WHERE a.id = application_id
        )
    ) INTO result;
    
    RETURN result;
END;
$$ LANGUAGE plpgsql;
```

---

## 9. REST API Specification

### 9.1 Component Endpoints

```
GET    /api/v1/components                    # List components (filtered by app, permissions)
GET    /api/v1/components/{slug}             # Get component definition
POST   /api/v1/components                    # Register new component
PUT    /api/v1/components/{slug}             # Update component
DELETE /api/v1/components/{slug}             # Deprecate component (soft delete)
GET    /api/v1/components/search?q=...     # Full-text search
POST   /api/v1/components/semantic-search    # Vector semantic search
GET    /api/v1/components/{slug}/bindings    # Get database bindings for component
GET    /api/v1/components/{slug}/examples    # Get usage examples
```

### 9.2 Application Endpoints

```
GET    /api/v1/applications                  # List applications
GET    /api/v1/applications/{slug}          # Get application config
POST   /api/v1/applications                # Create application
PUT    /api/v1/applications/{slug}          # Update application
DELETE /api/v1/applications/{slug}          # Archive application
GET    /api/v1/applications/{slug}/components # Get app-scoped components
GET    /api/v1/applications/{slug}/roles     # Get roles for application
POST   /api/v1/applications/{slug}/roles     # Create role
GET    /api/v1/applications/{slug}/design-system # Get design system
PUT    /api/v1/applications/{slug}/design-system # Set design system
```

### 9.3 Binding Endpoints

```
GET    /api/v1/bindings                      # List bindings
GET    /api/v1/bindings/{id}                 # Get binding config
POST   /api/v1/bindings                      # Create binding
PUT    /api/v1/bindings/{id}                 # Update binding
DELETE /api/v1/bindings/{id}                 # Delete binding
POST   /api/v1/bindings/auto-generate        # Trigger auto-generation for schema
GET    /api/v1/bindings/for-table/{schema}/{table} # Get binding for table
```

### 9.4 Design System Endpoints

```
GET    /api/v1/design-systems                # List design systems
GET    /api/v1/design-systems/{slug}        # Get design system
POST   /api/v1/design-systems               # Create design system
POST   /api/v1/design-systems/import        # Import ODSF bundle
POST   /api/v1/design-systems/{slug}/export # Export as ODSF bundle
GET    /api/v1/design-systems/{slug}/tokens   # Resolve tokens for component
```

### 9.5 Schema Endpoints

```
GET    /api/v1/schemas                       # List schemas
GET    /api/v1/schemas/{slug}               # Get schema definition
POST   /api/v1/schemas                      # Register schema
PUT    /api/v1/schemas/{slug}               # Update schema
POST   /api/v1/schemas/{slug}/validate      # Validate JSON against schema
GET    /api/v1/schemas/{slug}/ui-schema     # Get UI schema for data schema
POST   /api/v1/schemas/from-table/{schema}/{table} # Auto-generate from table
```

---

## 10. A2A Task Definitions

### 10.1 Task Catalog

The `flint-platform-agent` exposes these A2A tasks for registry management:

| Task ID | Description | Input | Output |
|---------|-------------|-------|--------|
| `a2ui.component.register` | Register a new component | Component JSON | Component ID |
| `a2ui.component.update` | Update component definition | Slug + delta JSON | Updated component |
| `a2ui.component.discover` | Find components by description | Natural language query | Component list |
| `a2ui.component.bind` | Bind component to DB object | Binding config | Binding ID |
| `a2ui.component.assemble` | Assemble component tree from event | Event type + context | A2UI JSON |
| `a2ui.application.create` | Create new application | App config | Application ID |
| `a2ui.application.configure` | Update app config | Slug + delta | Updated app |
| `a2ui.design_system.import` | Import ODSF bundle | Bundle URL | Design system ID |
| `a2ui.design_system.export` | Export as ODSF | Application ID | Bundle JSON |
| `a2ui.schema.generate` | Generate JSON Schema from table | Table reference | Schema JSON |
| `a2ui.schema.validate` | Validate data against schema | Data + schema slug | Validation result |
| `a2ui.binding.auto` | Auto-generate all bindings for app | Application ID | Binding list |
| `a2ui.surface.render` | Render a surface for an event | Event + context | A2UI surface JSON |
| `a2ui.surface.update` | Update a surface reactively | Surface ID + event | Updated A2UI |
| `a2ui.search.semantic` | Semantic search for components | Query text | Ranked components |
| `a2ui.token.resolve` | Resolve design tokens | App + component + user | Token map |

### 10.2 Task State Machine

```
Submitted → Working → InputRequired → Completed
                ↓
              Failed
                ↓
              Canceled
```

The `a2ui.component.assemble` task may enter `InputRequired` when the agent needs clarification on which component to use or how to bind data.

---

## 11. MCP Server Tools

### 11.1 Tool Manifest

The registry exposes these MCP tools:

```json
{
  "tools": [
    {
      "name": "a2ui_list_components",
      "description": "List available A2UI components filtered by category, application, or permissions",
      "parameters": {
        "category": { "type": "string", "enum": ["layout", "data-display", "input", "action", "agent"] },
        "application": { "type": "string" },
        "query": { "type": "string", "description": "Search query" }
      }
    },
    {
      "name": "a2ui_get_component",
      "description": "Get full component definition including schema, examples, and bindings",
      "parameters": {
        "slug": { "type": "string" }
      }
    },
    {
      "name": "a2ui_semantic_search",
      "description": "Find components using natural language description",
      "parameters": {
        "query": { "type": "string" },
        "limit": { "type": "integer", "default": 5 }
      }
    },
    {
      "name": "a2ui_generate_form",
      "description": "Generate a form component for a database table",
      "parameters": {
        "table_schema": { "type": "string" },
        "table_name": { "type": "string" },
        "operation": { "type": "string", "enum": ["create", "update", "view"] }
      }
    },
    {
      "name": "a2ui_generate_grid",
      "description": "Generate a data grid component for a database table or view",
      "parameters": {
        "table_schema": { "type": "string" },
        "table_name": { "type": "string" },
        "columns": { "type": "array", "items": { "type": "string" } }
      }
    },
    {
      "name": "a2ui_resolve_tokens",
      "description": "Resolve design tokens for an application and component",
      "parameters": {
        "application": { "type": "string" },
        "component": { "type": "string" }
      }
    },
    {
      "name": "a2ui_assemble_surface",
      "description": "Assemble an A2UI surface from an event and context",
      "parameters": {
        "event_type": { "type": "string" },
        "context": { "type": "object" },
        "application": { "type": "string" }
      }
    }
  ]
}
```

---

## 12. Event-Driven Dynamic Component Construction

### 12.1 The Event Assembly Pipeline

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     EVENT-DRIVEN ASSEMBLY PIPELINE                          │
│                                                                             │
│  ┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐           │
│  │   Event Source  │   │  Event Router   │   │  Component      │           │
│  │                 │   │  (flint_a2ui)   │   │  Assembler      │           │
│  │ • Agent infer   │──→│                 │──→│  (Rust engine)  │           │
│  │ • Tool call     │   │  Matches event  │   │                 │           │
│  │ • Skill activate│   │  type to        │   │  • Query registry│           │
│  │ • DB change     │   │  assembly rule  │   │  • Resolve perms│           │
│  │ • User action   │   │                 │   │  • Fetch tokens │           │
│  │ • Timer/schedule│   │  Rules stored   │   │  • Compose tree │           │
│  │                 │   │  in JSONB       │   │  • Emit A2UI    │           │
│  └─────────────────┘   └─────────────────┘   └─────────────────┘           │
│                              │                        │                      │
│                              ▼                        ▼                      │
│  ┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐           │
│  │  Assembly Rules │   │  Iggy Producer  │   │  Client Render  │           │
│  │  (flint_a2ui)   │   │  (topic: a2ui)  │   │  (Web/Desk/Mob) │           │
│  │                 │   │                 │   │                 │           │
│  │  event_type     │   │  Pushes A2UI    │   │  Receives A2UI  │           │
│  │  → component    │   │  surface JSON   │   │  JSON → native  │           │
│  │  → binding      │   │  to subscribers │   │  components     │           │
│  │  → token set    │   │                 │   │                 │           │
│  │  → permission   │   │                 │   │                 │           │
│  └─────────────────┘   └─────────────────┘   └─────────────────┘           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 12.2 Assembly Rules Schema

```sql
CREATE TABLE flint_a2ui.assembly_rules (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    application_id  uuid NOT NULL REFERENCES flint_a2ui.applications(id),
    
    -- Event matching
    event_type      text NOT NULL,
    event_filter    jsonb NOT NULL DEFAULT '{}',  -- JSONPath-like filter
    -- Example: { "tool_name": "generate_report", "status": "completed" }
    
    -- Assembly configuration
    assembly_config jsonb NOT NULL,
    -- Example: {
    --   "surface_type": "modal",
    --   "root_component": "agent-chat",
    --   "components": [
    --     { "ref": "streaming-text", "bind": "event.result.text" },
    --     { "ref": "action-bar", "actions": ["copy", "save", "share"] }
    --   ],
    --   "data_bindings": { "result": "event.result", "context": "event.context" }
    -- }
    
    -- Priority (higher = evaluated first)
    priority        int NOT NULL DEFAULT 100,
    
    -- Active flag
    is_active       boolean NOT NULL DEFAULT true,
    
    -- Metadata
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now()
);
```

### 12.3 Example: Tool Call Completion → Component Assembly

When an MCP tool call completes, the event flows through the assembly pipeline:

```json
{
  "event_type": "tool_call_completed",
  "source": "mcp_tool",
  "source_id": "tool-a2ui_generate_grid",
  "payload": {
    "tool_name": "a2ui_generate_grid",
    "status": "success",
    "result": {
      "component": "data-grid",
      "config": {
        "data_source": "public.customers",
        "columns": ["id", "name", "email", "status"]
      }
    }
  },
  "application_id": "my-app-uuid",
  "user_id": "user-uuid"
}
```

The assembly rule matches and generates:

```json
{
  "surface_update": {
    "surface_id": "main-view",
    "components": [
      {
        "type": "data-grid",
        "id": "customer-grid",
        "props": {
          "data_source": "public.customers",
          "columns": [
            { "field": "id", "header": "ID", "width": "80px" },
            { "field": "name", "header": "Name", "sortable": true },
            { "field": "email", "header": "Email", "component": "email-link" },
            { "field": "status", "header": "Status", "component": "badge" }
          ],
          "row_actions": ["view", "edit"],
          "bulk_actions": ["delete-selected"]
        },
        "tokens": {
          "accent": "#FF6A3D",
          "surface": "#131A22"
        }
      }
    ]
  }
}
```

---

## 13. Security and Permissions

### 13.1 Threat Model

| Threat | Mitigation |
|--------|------------|
| **Unauthorized component registration** | Only `admin` roles can register components. All registrations are audited. |
| **Component impersonation** | Slug uniqueness enforced at database level. Components cannot shadow base components. |
| **Embedding injection** | Source text is sanitized before embedding. No executable code in descriptions. |
| **Design token poisoning** | Token resolution is scoped to application. Cross-app token leakage prevented by RLS. |
| **Binding to unauthorized tables** | Bindings respect RLS policies. `flint_meta` cache tables are not directly exposed. |
| **A2UI injection attacks** | Constrained generation: only pre-approved components. Unconstrained mode requires explicit opt-in and sandboxed iframe. |
| **JWT claim forgery** | JWT signed by Kratos. Claims verified by `flint-gate` before reaching registry. |
| **Role escalation** | Role assignments are audited. Parent role inheritance is checked at assignment time. |
| **Event log tampering** | `flint_a2ui.events` is append-only. No UPDATE/DELETE allowed (enforced by RLS). |

### 13.2 Row-Level Security

```sql
-- Components: users can only see components in their applications or base components
ALTER TABLE flint_a2ui.components ENABLE ROW LEVEL SECURITY;

CREATE POLICY component_access ON flint_a2ui.components
    FOR ALL
    USING (
        is_base = true
        OR application_id IS NULL
        OR application_id IN (
            SELECT application_id FROM flint_a2ui.role_assignments
            WHERE user_id = current_setting('app.jwt_claims')::jsonb->'flint'->>'user_id'
        )
    );

-- Events: users can only see events from their applications
ALTER TABLE flint_a2ui.events ENABLE ROW LEVEL SECURITY;

CREATE POLICY event_access ON flint_a2ui.events
    FOR SELECT
    USING (
        application_id IN (
            SELECT application_id FROM flint_a2ui.role_assignments
            WHERE user_id = current_setting('app.jwt_claims')::jsonb->'flint'->>'user_id'
        )
        OR application_id IS NULL
    );
```

### 13.3 Audit Trail

All security-relevant actions are logged to `flint_a2ui.events` and `flint_meta.audit_log`:

```sql
-- Cross-reference audit log
CREATE TABLE flint_a2ui.audit_log (
    LIKE flint_meta.audit_log INCLUDING ALL,
    PRIMARY KEY (id)
);

-- Additional A2UI-specific events
INSERT INTO flint_a2ui.audit_log (event_type, actor, object, action, result, details)
VALUES 
    ('component_registered', 'admin-uuid', 'data-grid', 'create', true, '{"version": "1.2.0"}'::jsonb),
    ('binding_auto_generated', 'system', 'public.customers', 'create', true, '{"confidence": 0.95}'::jsonb),
    ('design_system_imported', 'admin-uuid', 'odsf-linear', 'import', true, '{"source": "open-design.ai"}'::jsonb);
```

---

## 14. Implementation Roadmap

### 14.1 Milestone 1: Core Registry (Weeks 1-4)

**Deliverables:**
- Database schema for `flint_a2ui` (all tables defined in this document)
- Base component primitives (layout + data-display + input + action)
- REST API for CRUD operations
- JSONB schema validation
- Basic text search

**Exit Criteria:**
- Components can be registered, queried, and retrieved via REST
- Base applications (`flint-admin`, `flint-playground`) created
- RLS policies active
- 50+ base components defined

### 14.2 Milestone 2: Semantic Search and Embeddings (Weeks 5-7)

**Deliverables:**
- `pgvector` integration
- Embedding generation pipeline (Rust + OpenAI/text-embedding-3-large)
- Semantic search API (`POST /components/semantic-search`)
- Hybrid search (BM25 + vector similarity)
- Auto-embedding on component registration/update

**Exit Criteria:**
- Natural language queries return relevant components
- Search latency < 100ms for 10k components
- Embeddings regenerate on schema changes

### 14.3 Milestone 3: Database Binding and Auto-Generation (Weeks 8-11)

**Deliverables:**
- `flint_a2ui.bindings` table with auto-generation triggers
- `flint_meta` → `flint_a2ui` integration (event triggers)
- Column type → component mapping
- Table → form/grid auto-generation
- Function → action component mapping

**Exit Criteria:**
- New table in `flint_meta` auto-generates binding within 5 seconds
- Form component generated for any table with 95% accuracy
- Grid component generated for any table with 100% accuracy

### 14.4 Milestone 4: Application Model and Permissions (Weeks 12-14)

**Deliverables:**
- Application CRUD with ownership
- Role and permission system
- JWT claims template resolution
- Application-scoped component catalogs
- Permission-filtered component queries

**Exit Criteria:**
- User sees only components they have permission to view
- Application isolation enforced at database level
- Role hierarchy works (parent → child inheritance)

### 14.5 Milestone 5: Design System Integration (Weeks 15-17)

**Deliverables:**
- ODSF import pipeline
- Open Design bridge (`import_odsf` function)
- Token resolution at runtime
- Component token mapping
- Export to ODSF format

**Exit Criteria:**
- Open Design `DESIGN.md` can be imported
- Design tokens resolve per application + component
- CSS generation for web and desktop platforms

### 14.6 Milestone 6: Event-Driven Assembly (Weeks 18-20)

**Deliverables:**
- `flint_a2ui.assembly_rules` table
- Event routing engine
- Component assembler (Rust)
- Iggy integration for real-time push
- A2UI surface generation from events

**Exit Criteria:**
- Tool call completion generates A2UI surface within 500ms
- Database change triggers UI update via WebSocket
- Assembly rules are configurable per application

### 14.7 Milestone 7: Protocol Surfaces (Weeks 21-23)

**Deliverables:**
- A2A task definitions for all registry operations
- MCP tool server for component discovery
- AG-UI streaming surface for component rendering
- A2UI JSON output generation
- `flint-platform-agent` integration

**Exit Criteria:**
- Claude Desktop can discover and use Flint components via MCP
- A2A task can assemble a surface from a tool call
- AG-UI stream delivers component updates in real-time

### 14.8 Milestone 8: Federation and Scale (Weeks 24-26)

**Deliverables:**
- CRDT-based component sync across nodes
- Multi-tenant embedding isolation
- Component versioning with rollback
- CDN distribution for static assets
- Performance benchmarking

**Exit Criteria:**
- 100k components with < 50ms search latency
- Cross-node component sync with conflict resolution
- 99.9% availability for registry queries

---

## 15. Integration with Flint Ecosystem

### 15.1 Integration Matrix

| System | Integration Point | Mechanism |
|--------|-------------------|-----------|
| **flint-gate** | JWT claims, auth proxy | `SET LOCAL app.jwt_claims` → permission resolution |
| **flint-forge** | Database metadata, reflection | `flint_meta` → `flint_a2ui` event triggers |
| **flint-realtime-fabric** | Real-time UI updates | Iggy topics: `a2ui.events`, `a2ui.surfaces` |
| **flint-platform-agent** | Administrative interface | A2A tasks, MCP tools, REST API |
| **flint-vault** | Encrypted column rendering | `x-encrypted` flag → Vault key resolution |
| **Open Design** | Design system import/export | ODSF bridge, `DESIGN.md` parser |
| **pg_graphql** | GraphQL schema introspection | Component bindings from GraphQL types |
| **Keto** | Permission checks | `keto_namespace` + `keto_relation` in roles |
| **Kratos** | Identity + JWT minting | User identity → JWT claims template |
| **Cedar** | Policy evaluation | Cedar policies at component access boundary |

### 15.2 Data Flow Summary

| Step | From | To | Action |
|------|------|-----|--------|
| 1 | Database DDL | flint_meta | Event trigger updates cache tables |
| 2 | flint_meta | flint_a2ui | Auto-generate component bindings |
| 3 | flint_a2ui | Iggy | NOTIFY → Iggy Producer pushes event |
| 4 | Iggy | Client | WebSocket delivers A2UI surface update |
| 5 | User | flint-gate | Authenticates, receives JWT |
| 6 | flint-gate | flint_a2ui | JWT claims resolve component visibility |
| 7 | Agent | flint-platform-agent | MCP tool discovers component |
| 8 | Agent | flint_a2ui | Semantic search finds component |
| 9 | Agent | flint_a2ui | Assembles surface from event |
| 10 | Open Design | flint_a2ui | ODSF import → design system record |
| 11 | flint_a2ui | Open Design | Export → ODSF bundle for sharing |

---

## 16. Recommendations and Future Work

### 16.1 What We Might Be Missing

Based on research and industry patterns, consider adding:

1. **Component test harness** — Each component should have a `test_config` JSONB that defines how to test it (input data, expected output, accessibility checks). This enables automated regression testing of generated UIs.

2. **Multi-modal embeddings** — Store screenshots/thumbnails of components alongside text embeddings. Agents can search by visual similarity: *"find me a component that looks like this screenshot."*

3. **Component composition constraints** — Some components can only contain certain children (e.g., `Accordion` only contains `AccordionItem`). Add a `composition_rules` JSONB to enforce valid component trees.

4. **Animation and transition definitions** — A2UI v1.0 adds `Surface Properties` for theming. Consider adding `animation` JSONB for enter/exit/transition animations per component.

5. **State machine definitions** — For interactive components (`Wizard`, `Stepper`, `Form`), define the state machine in JSONB so agents understand valid transitions.

6. **Locale and internationalization** — Component definitions should support `i18n` JSONB with translations for labels, placeholders, and aria-labels.

7. **Dark mode and theme variants** — Design tokens should support `light`/`dark`/`high-contrast` variants, not just a single token set.

8. **Component performance budgets** — Add `performance` JSONB with load time, render time, and bundle size targets so agents can optimize for constrained environments.

9. **Accessibility compliance engine** — Automated A11y checks on component definitions (contrast ratios, keyboard navigation, screen reader labels) before registration approval.

10. **Component marketplace** — A discovery layer where users can browse, rate, and install community components into their applications.

### 16.2 Why This Structure Is Important

The registry is structured this way for these reasons:

1. **Database as source of truth** — All metadata lives in PostgreSQL. No external caches to invalidate. Agent queries go directly to the data layer.

2. **JSONB for extensibility** — The schema can evolve without migrations. New component properties, new token types, new permission dimensions are just new JSON keys.

3. **Embeddings for AI-native discovery** — Agents don't search by keyword; they search by intent. Vector embeddings make the registry naturally agent-friendly.

4. **Database binding for automation** — When a table is created, the UI for it is automatically available. This is not code generation; it's metadata-driven runtime assembly.

5. **Application scoping for multi-tenancy** — Every application is isolated but shares base components. This enables SaaS deployment where each tenant has custom components but inherits platform primitives.

6. **Event-driven for real-time** — The UI is not static. It reassembles in response to events. This is essential for agent-driven interfaces where the UI is a function of the agent's state.

7. **Open Design bridge for creative workflow** — Designers work in Open Design. Developers work in Flint. The bridge ensures design intent flows into the runtime without manual translation.

8. **JWT-propagated for security** — Identity flows from the gateway to the database. Component visibility is not an application-layer concern; it's a data-layer concern enforced by RLS.

---

**Document ID:** RFC-FORGE-A2UI-001  
**Version:** 1.0  
**Date:** June 2026  
**Status:** Architecture Design — Ready for Implementation
