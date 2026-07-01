# p5-c002 — Base Components Seed

**Phase:** 5 — Flint A2UI Component Registry  
**Priority:** P0  
**Depends on:** p5-c001 (flint_a2ui schema)  
**Blocks:** p7-c005a (CopilotKit catalog endpoint)

---

## What this change delivers

Seeds 50+ Flint base component definitions into `flint_a2ui.components`. These are **Flint-invented component definitions** conforming to the A2UI v0.9.1 component type model. The official A2UI Basic Catalog has only Text, Button, Row — everything else here is original Flint work.

Components are seeded from `scripts/seed_a2ui_components.sql`, which is idempotent (`INSERT ... ON CONFLICT (slug) DO UPDATE`).

### Component categories and count

| Category | Count | Examples |
|---|---|---|
| `layout` | 8 | container, row, column, grid, stack, divider, spacer, scroll-area |
| `data-display` | 12 | data-grid, data-table, text, badge, tag, avatar, stat-card, timeline, code-block, json-viewer, list, detail-view |
| `input` | 14 | form, text-input, number-input, select, multi-select, date-picker, checkbox, radio, toggle, textarea, file-upload, search-input, color-picker, slider |
| `action` | 6 | button, action-bar, dropdown-menu, context-menu, fab, link |
| `navigation` | 6 | nav-bar, sidebar, tabs, breadcrumb, pagination, stepper |
| `feedback` | 8 | alert, toast, modal, dialog, loading-spinner, progress-bar, empty-state, error-boundary |
| `system` | 1 | `flint-meta-schema` (self-registration of flint_meta.agui_descriptor output) |

**Total: 55 base components**

### Seed format (abbreviated example)

```sql
INSERT INTO flint_a2ui.components
    (slug, category, primitive_type, schema, is_base, description, usage_examples, design_tokens)
VALUES
    (
        'data-grid',
        'data-display',
        'DataGrid',
        '{
            "type": "object",
            "properties": {
                "data_source": { "type": "string", "description": "Table slug or MCP tool name" },
                "columns": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "field": { "type": "string" },
                            "header": { "type": "string" },
                            "sortable": { "type": "boolean", "default": false },
                            "component": { "type": "string", "description": "Inline component slug for cell rendering" }
                        },
                        "required": ["field", "header"]
                    }
                },
                "row_actions": { "type": "array", "items": { "type": "string" } },
                "bulk_actions": { "type": "array", "items": { "type": "string" } },
                "pagination": { "type": "boolean", "default": true }
            },
            "required": ["data_source", "columns"]
        }',
        true,
        'A sortable, filterable data grid for displaying tabular data from a database table or MCP tool.',
        '[{
            "context": "Display orders table",
            "config": {
                "data_source": "public.orders",
                "columns": [
                    { "field": "id", "header": "ID" },
                    { "field": "status", "header": "Status", "component": "badge" },
                    { "field": "total", "header": "Total" }
                ],
                "row_actions": ["view", "edit"]
            }
        }]',
        '{ "accent": "var(--color-accent)", "surface": "var(--color-surface)", "border": "var(--color-border)" }'
    )
ON CONFLICT (slug) DO UPDATE
    SET schema = EXCLUDED.schema,
        description = EXCLUDED.description,
        usage_examples = EXCLUDED.usage_examples,
        design_tokens = EXCLUDED.design_tokens,
        updated_at = now();
```

### Special component: `flint-meta-schema` (system category)

This component represents the output of `flint_meta.agui_descriptor()` registered as a system component in the registry. It is NOT rendered by frontends — it is the self-registration of the Flint schema metadata surface in the component catalog so agents can discover it via `a2ui_list_components`.

```sql
INSERT INTO flint_a2ui.components
    (slug, category, primitive_type, schema, is_base, description)
VALUES
    (
        'flint-meta-schema',
        'system',
        'SchemaDescriptor',
        '{ "type": "object", "description": "Flint schema metadata descriptor from flint_meta.agui_descriptor()" }',
        true,
        'System component: represents the Flint database schema metadata surface. Not rendered by frontends. '
        'Used by agents to discover available tables, functions, and capabilities via the MCP/A2A protocol.'
    )
ON CONFLICT (slug) DO NOTHING;
```

---

## Gate tests

- `SELECT COUNT(*) FROM flint_a2ui.components WHERE is_base = true` ≥ 50
- All 7 categories represented
- `flint-meta-schema` system component exists
- Schema JSONB is valid JSON Schema (check via `jsonb_typeof(schema) = 'object'` and `schema ? 'type'`)
- `data-grid`, `form`, `text-input`, `button`, `modal`, `nav-bar` all present
