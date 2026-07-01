-- Seed: Flint A2UI base component catalog (55 components)
-- Idempotent: ON CONFLICT (slug) DO UPDATE
-- Run after migrations/0002_flint_a2ui.sql is applied.
-- These are Flint-invented component definitions conforming to the A2UI v0.9.1 type model.
-- The official A2UI Basic Catalog contains only Text, Button, Row.

-- ============================================================
-- LAYOUT (8)
-- ============================================================

INSERT INTO flint_a2ui.components
    (slug, category, primitive_type, schema, is_base, description, usage_examples, design_tokens)
VALUES
    (
        'container',
        'layout',
        'Container',
        '{
            "type": "object",
            "properties": {
                "max_width": { "type": "string", "default": "1280px" },
                "padding": { "type": "string", "default": "var(--space-md)" },
                "centered": { "type": "boolean", "default": true },
                "children": { "type": "array", "items": { "type": "object" } }
            }
        }',
        true,
        'Top-level layout container with configurable max-width and padding.',
        '[{"context": "Page wrapper", "config": {"max_width": "960px", "centered": true}}]',
        '{"max_width": "var(--layout-max-width)", "padding": "var(--space-md)"}'
    ),
    (
        'row',
        'layout',
        'Row',
        '{
            "type": "object",
            "properties": {
                "gap": { "type": "string", "default": "var(--space-sm)" },
                "align": { "type": "string", "enum": ["start", "center", "end", "stretch"], "default": "center" },
                "justify": { "type": "string", "enum": ["start", "center", "end", "between", "around"], "default": "start" },
                "wrap": { "type": "boolean", "default": false },
                "children": { "type": "array", "items": { "type": "object" } }
            }
        }',
        true,
        'Horizontal flex row layout.',
        '[{"context": "Toolbar", "config": {"gap": "var(--space-sm)", "align": "center", "justify": "between"}}]',
        '{"gap": "var(--space-sm)"}'
    ),
    (
        'column',
        'layout',
        'Column',
        '{
            "type": "object",
            "properties": {
                "gap": { "type": "string", "default": "var(--space-sm)" },
                "align": { "type": "string", "enum": ["start", "center", "end", "stretch"], "default": "stretch" },
                "children": { "type": "array", "items": { "type": "object" } }
            }
        }',
        true,
        'Vertical flex column layout.',
        '[{"context": "Form section", "config": {"gap": "var(--space-md)", "align": "stretch"}}]',
        '{"gap": "var(--space-sm)"}'
    ),
    (
        'grid',
        'layout',
        'Grid',
        '{
            "type": "object",
            "properties": {
                "columns": { "oneOf": [{"type": "integer", "minimum": 1}, {"type": "string"}], "default": 12 },
                "gap": { "type": "string", "default": "var(--space-md)" },
                "row_gap": { "type": "string" },
                "children": { "type": "array", "items": { "type": "object" } }
            }
        }',
        true,
        'CSS grid layout with configurable columns and gap.',
        '[{"context": "Dashboard cards", "config": {"columns": 3, "gap": "var(--space-lg)"}}]',
        '{"gap": "var(--space-md)"}'
    ),
    (
        'stack',
        'layout',
        'Stack',
        '{
            "type": "object",
            "properties": {
                "direction": { "type": "string", "enum": ["vertical", "horizontal"], "default": "vertical" },
                "spacing": { "type": "string", "default": "var(--space-sm)" },
                "dividers": { "type": "boolean", "default": false },
                "children": { "type": "array", "items": { "type": "object" } }
            }
        }',
        true,
        'Evenly spaced stack of children with optional dividers.',
        '[{"context": "Settings list", "config": {"direction": "vertical", "spacing": "var(--space-sm)", "dividers": true}}]',
        '{"spacing": "var(--space-sm)"}'
    ),
    (
        'divider',
        'layout',
        'Divider',
        '{
            "type": "object",
            "properties": {
                "orientation": { "type": "string", "enum": ["horizontal", "vertical"], "default": "horizontal" },
                "label": { "type": "string" },
                "thickness": { "type": "string", "default": "1px" }
            }
        }',
        true,
        'Visual separator between sections.',
        '[{"context": "Section break", "config": {"orientation": "horizontal", "label": "Or"}}]',
        '{"color": "var(--color-border)", "thickness": "1px"}'
    ),
    (
        'spacer',
        'layout',
        'Spacer',
        '{
            "type": "object",
            "properties": {
                "size": { "type": "string", "default": "var(--space-md)" },
                "axis": { "type": "string", "enum": ["vertical", "horizontal", "both"], "default": "vertical" }
            }
        }',
        true,
        'Flexible whitespace for explicit spacing.',
        '[{"context": "Header gap", "config": {"size": "var(--space-xl)", "axis": "vertical"}}]',
        '{}'
    ),
    (
        'scroll-area',
        'layout',
        'ScrollArea',
        '{
            "type": "object",
            "properties": {
                "height": { "type": "string" },
                "max_height": { "type": "string", "default": "400px" },
                "scrollbar": { "type": "string", "enum": ["auto", "always", "hidden"], "default": "auto" },
                "children": { "type": "array", "items": { "type": "object" } }
            }
        }',
        true,
        'Overflow container with styled scrollbar.',
        '[{"context": "Log viewer", "config": {"max_height": "300px", "scrollbar": "always"}}]',
        '{"scrollbar_color": "var(--color-muted)"}'
    )
ON CONFLICT (slug) DO UPDATE
    SET schema = EXCLUDED.schema,
        description = EXCLUDED.description,
        usage_examples = EXCLUDED.usage_examples,
        design_tokens = EXCLUDED.design_tokens,
        updated_at = now();

-- ============================================================
-- DATA-DISPLAY (12)
-- ============================================================

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
                            "component": { "type": "string" }
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
        'Sortable, filterable data grid for tabular data from a database table or MCP tool.',
        '[{"context": "Orders table", "config": {"data_source": "public.orders", "columns": [{"field": "id", "header": "ID"}, {"field": "status", "header": "Status", "component": "badge"}], "row_actions": ["view", "edit"]}}]',
        '{"accent": "var(--color-accent)", "surface": "var(--color-surface)", "border": "var(--color-border)"}'
    ),
    (
        'data-table',
        'data-display',
        'DataTable',
        '{
            "type": "object",
            "properties": {
                "data": { "type": "array", "items": { "type": "object" } },
                "columns": { "type": "array", "items": { "type": "object", "properties": {"field": {"type": "string"}, "header": {"type": "string"}}, "required": ["field", "header"] } },
                "striped": { "type": "boolean", "default": true },
                "hover": { "type": "boolean", "default": true }
            },
            "required": ["data", "columns"]
        }',
        true,
        'Static data table for in-memory arrays.',
        '[{"context": "Summary results", "config": {"data": [], "columns": [{"field": "name", "header": "Name"}], "striped": true}}]',
        '{"stripe_color": "var(--color-surface-alt)"}'
    ),
    (
        'text',
        'data-display',
        'Text',
        '{
            "type": "object",
            "properties": {
                "content": { "type": "string" },
                "variant": { "type": "string", "enum": ["body", "heading", "caption", "code", "muted"], "default": "body" },
                "size": { "type": "string", "enum": ["xs", "sm", "md", "lg", "xl", "2xl"], "default": "md" },
                "weight": { "type": "string", "enum": ["normal", "medium", "semibold", "bold"], "default": "normal" },
                "truncate": { "type": "boolean", "default": false }
            },
            "required": ["content"]
        }',
        true,
        'Typographic text element with variant and size control.',
        '[{"context": "Page heading", "config": {"content": "Dashboard", "variant": "heading", "size": "2xl", "weight": "bold"}}]',
        '{"color": "var(--color-text)", "muted": "var(--color-muted)"}'
    ),
    (
        'badge',
        'data-display',
        'Badge',
        '{
            "type": "object",
            "properties": {
                "label": { "type": "string" },
                "variant": { "type": "string", "enum": ["default", "success", "warning", "error", "info"], "default": "default" },
                "size": { "type": "string", "enum": ["sm", "md"], "default": "sm" }
            },
            "required": ["label"]
        }',
        true,
        'Status badge with semantic color variants.',
        '[{"context": "Order status", "config": {"label": "Active", "variant": "success"}}]',
        '{"success": "var(--color-success)", "warning": "var(--color-warning)", "error": "var(--color-error)"}'
    ),
    (
        'tag',
        'data-display',
        'Tag',
        '{
            "type": "object",
            "properties": {
                "label": { "type": "string" },
                "removable": { "type": "boolean", "default": false },
                "on_remove": { "type": "string", "description": "Action ID" }
            },
            "required": ["label"]
        }',
        true,
        'Removable tag/chip for categorization.',
        '[{"context": "Filter chip", "config": {"label": "Active", "removable": true, "on_remove": "remove_filter"}}]',
        '{"bg": "var(--color-surface-alt)", "border": "var(--color-border)"}'
    ),
    (
        'avatar',
        'data-display',
        'Avatar',
        '{
            "type": "object",
            "properties": {
                "src": { "type": "string" },
                "alt": { "type": "string" },
                "initials": { "type": "string" },
                "size": { "type": "string", "enum": ["xs", "sm", "md", "lg", "xl"], "default": "md" },
                "shape": { "type": "string", "enum": ["circle", "square"], "default": "circle" }
            }
        }',
        true,
        'User avatar with image or initials fallback.',
        '[{"context": "User profile", "config": {"initials": "TJ", "size": "md", "shape": "circle"}}]',
        '{"bg": "var(--color-accent)", "text": "var(--color-on-accent)"}'
    ),
    (
        'stat-card',
        'data-display',
        'StatCard',
        '{
            "type": "object",
            "properties": {
                "label": { "type": "string" },
                "value": { "type": "string" },
                "change": { "type": "string", "description": "e.g. +12%" },
                "trend": { "type": "string", "enum": ["up", "down", "neutral"] },
                "icon": { "type": "string" }
            },
            "required": ["label", "value"]
        }',
        true,
        'KPI stat card with value, label, and trend indicator.',
        '[{"context": "Revenue card", "config": {"label": "Revenue", "value": "$12,400", "change": "+8%", "trend": "up"}}]',
        '{"up": "var(--color-success)", "down": "var(--color-error)"}'
    ),
    (
        'timeline',
        'data-display',
        'Timeline',
        '{
            "type": "object",
            "properties": {
                "items": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "timestamp": { "type": "string" },
                            "title": { "type": "string" },
                            "description": { "type": "string" },
                            "icon": { "type": "string" }
                        },
                        "required": ["timestamp", "title"]
                    }
                },
                "direction": { "type": "string", "enum": ["vertical", "horizontal"], "default": "vertical" }
            },
            "required": ["items"]
        }',
        true,
        'Chronological event timeline.',
        '[{"context": "Order history", "config": {"items": [{"timestamp": "2024-01-01", "title": "Order placed"}]}}]',
        '{"connector": "var(--color-border)", "dot": "var(--color-accent)"}'
    ),
    (
        'code-block',
        'data-display',
        'CodeBlock',
        '{
            "type": "object",
            "properties": {
                "code": { "type": "string" },
                "language": { "type": "string", "default": "text" },
                "copyable": { "type": "boolean", "default": true },
                "line_numbers": { "type": "boolean", "default": false },
                "max_height": { "type": "string" }
            },
            "required": ["code"]
        }',
        true,
        'Syntax-highlighted code block with copy button.',
        '[{"context": "API response", "config": {"code": "{\"id\": 1}", "language": "json", "copyable": true}}]',
        '{"bg": "var(--color-code-bg)", "text": "var(--color-code-text)"}'
    ),
    (
        'json-viewer',
        'data-display',
        'JsonViewer',
        '{
            "type": "object",
            "properties": {
                "data": {},
                "collapsed": { "type": "integer", "description": "Default collapse depth", "default": 1 },
                "copyable": { "type": "boolean", "default": true }
            },
            "required": ["data"]
        }',
        true,
        'Interactive JSON tree viewer with collapse/expand.',
        '[{"context": "Debug payload", "config": {"data": {}, "collapsed": 2}}]',
        '{"key": "var(--color-accent)", "string": "var(--color-success)"}'
    ),
    (
        'list',
        'data-display',
        'List',
        '{
            "type": "object",
            "properties": {
                "items": { "type": "array", "items": { "type": "object" } },
                "item_component": { "type": "string", "description": "Component slug for rendering each item" },
                "ordered": { "type": "boolean", "default": false },
                "dividers": { "type": "boolean", "default": true }
            },
            "required": ["items"]
        }',
        true,
        'List of items rendered via a child component.',
        '[{"context": "Notification list", "config": {"items": [], "item_component": "detail-view", "dividers": true}}]',
        '{}'
    ),
    (
        'detail-view',
        'data-display',
        'DetailView',
        '{
            "type": "object",
            "properties": {
                "fields": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "label": { "type": "string" },
                            "value": {},
                            "component": { "type": "string" }
                        },
                        "required": ["label", "value"]
                    }
                },
                "layout": { "type": "string", "enum": ["vertical", "horizontal", "grid"], "default": "vertical" }
            },
            "required": ["fields"]
        }',
        true,
        'Key-value detail view for a single record.',
        '[{"context": "Order detail", "config": {"fields": [{"label": "Status", "value": "Active", "component": "badge"}], "layout": "grid"}}]',
        '{"label_color": "var(--color-muted)"}'
    )
ON CONFLICT (slug) DO UPDATE
    SET schema = EXCLUDED.schema,
        description = EXCLUDED.description,
        usage_examples = EXCLUDED.usage_examples,
        design_tokens = EXCLUDED.design_tokens,
        updated_at = now();

-- ============================================================
-- INPUT (14)
-- ============================================================

INSERT INTO flint_a2ui.components
    (slug, category, primitive_type, schema, is_base, description, usage_examples, design_tokens)
VALUES
    (
        'form',
        'input',
        'Form',
        '{
            "type": "object",
            "properties": {
                "fields": { "type": "array", "items": { "type": "object" } },
                "submit_action": { "type": "string", "description": "Action ID called on submit" },
                "submit_label": { "type": "string", "default": "Submit" },
                "layout": { "type": "string", "enum": ["vertical", "horizontal", "grid"], "default": "vertical" },
                "validation": { "type": "object", "additionalProperties": { "type": "object" } }
            },
            "required": ["fields", "submit_action"]
        }',
        true,
        'Form container with validation and submit action.',
        '[{"context": "Create user form", "config": {"fields": [], "submit_action": "create_user", "submit_label": "Create"}}]',
        '{"gap": "var(--space-md)"}'
    ),
    (
        'text-input',
        'input',
        'TextInput',
        '{
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "label": { "type": "string" },
                "placeholder": { "type": "string" },
                "type": { "type": "string", "enum": ["text", "email", "password", "url", "tel"], "default": "text" },
                "required": { "type": "boolean", "default": false },
                "disabled": { "type": "boolean", "default": false },
                "hint": { "type": "string" }
            },
            "required": ["name"]
        }',
        true,
        'Single-line text input field.',
        '[{"context": "Email field", "config": {"name": "email", "label": "Email", "type": "email", "required": true}}]',
        '{"border": "var(--color-border)", "focus": "var(--color-accent)"}'
    ),
    (
        'number-input',
        'input',
        'NumberInput',
        '{
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "label": { "type": "string" },
                "min": { "type": "number" },
                "max": { "type": "number" },
                "step": { "type": "number", "default": 1 },
                "required": { "type": "boolean", "default": false }
            },
            "required": ["name"]
        }',
        true,
        'Numeric input with min/max/step constraints.',
        '[{"context": "Quantity field", "config": {"name": "qty", "label": "Quantity", "min": 1, "max": 999}}]',
        '{"border": "var(--color-border)"}'
    ),
    (
        'select',
        'input',
        'Select',
        '{
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "label": { "type": "string" },
                "options": { "type": "array", "items": { "type": "object", "properties": {"label": {"type": "string"}, "value": {"type": "string"}}, "required": ["label", "value"] } },
                "placeholder": { "type": "string", "default": "Select..." },
                "required": { "type": "boolean", "default": false }
            },
            "required": ["name", "options"]
        }',
        true,
        'Single-value dropdown select.',
        '[{"context": "Status filter", "config": {"name": "status", "label": "Status", "options": [{"label": "Active", "value": "active"}, {"label": "Inactive", "value": "inactive"}]}}]',
        '{"border": "var(--color-border)"}'
    ),
    (
        'multi-select',
        'input',
        'MultiSelect',
        '{
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "label": { "type": "string" },
                "options": { "type": "array", "items": { "type": "object", "properties": {"label": {"type": "string"}, "value": {"type": "string"}}, "required": ["label", "value"] } },
                "searchable": { "type": "boolean", "default": true },
                "max_selected": { "type": "integer" }
            },
            "required": ["name", "options"]
        }',
        true,
        'Multi-value select with optional search and chip display.',
        '[{"context": "Tags field", "config": {"name": "tags", "label": "Tags", "options": [], "searchable": true}}]',
        '{"chip_bg": "var(--color-surface-alt)"}'
    ),
    (
        'date-picker',
        'input',
        'DatePicker',
        '{
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "label": { "type": "string" },
                "mode": { "type": "string", "enum": ["single", "range", "multiple"], "default": "single" },
                "min_date": { "type": "string", "format": "date" },
                "max_date": { "type": "string", "format": "date" },
                "required": { "type": "boolean", "default": false }
            },
            "required": ["name"]
        }',
        true,
        'Calendar date picker with single, range, or multiple modes.',
        '[{"context": "Filter date range", "config": {"name": "date_range", "label": "Date Range", "mode": "range"}}]',
        '{"selected_bg": "var(--color-accent)", "today_border": "var(--color-accent)"}'
    ),
    (
        'checkbox',
        'input',
        'Checkbox',
        '{
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "label": { "type": "string" },
                "checked": { "type": "boolean", "default": false },
                "indeterminate": { "type": "boolean", "default": false },
                "required": { "type": "boolean", "default": false }
            },
            "required": ["name", "label"]
        }',
        true,
        'Single checkbox with optional indeterminate state.',
        '[{"context": "Agree to terms", "config": {"name": "agree", "label": "I agree to the terms", "required": true}}]',
        '{"checked_bg": "var(--color-accent)"}'
    ),
    (
        'radio',
        'input',
        'RadioGroup',
        '{
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "label": { "type": "string" },
                "options": { "type": "array", "items": { "type": "object", "properties": {"label": {"type": "string"}, "value": {"type": "string"}}, "required": ["label", "value"] } },
                "direction": { "type": "string", "enum": ["vertical", "horizontal"], "default": "vertical" },
                "required": { "type": "boolean", "default": false }
            },
            "required": ["name", "options"]
        }',
        true,
        'Radio button group for mutually exclusive selection.',
        '[{"context": "Plan choice", "config": {"name": "plan", "label": "Plan", "options": [{"label": "Free", "value": "free"}, {"label": "Pro", "value": "pro"}]}}]',
        '{"checked_color": "var(--color-accent)"}'
    ),
    (
        'toggle',
        'input',
        'Toggle',
        '{
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "label": { "type": "string" },
                "checked": { "type": "boolean", "default": false },
                "size": { "type": "string", "enum": ["sm", "md", "lg"], "default": "md" }
            },
            "required": ["name", "label"]
        }',
        true,
        'Boolean toggle switch.',
        '[{"context": "Notifications toggle", "config": {"name": "notifications", "label": "Enable notifications"}}]',
        '{"on_bg": "var(--color-accent)", "off_bg": "var(--color-muted)"}'
    ),
    (
        'textarea',
        'input',
        'Textarea',
        '{
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "label": { "type": "string" },
                "placeholder": { "type": "string" },
                "rows": { "type": "integer", "default": 4 },
                "max_length": { "type": "integer" },
                "auto_resize": { "type": "boolean", "default": false },
                "required": { "type": "boolean", "default": false }
            },
            "required": ["name"]
        }',
        true,
        'Multi-line text area with optional auto-resize.',
        '[{"context": "Notes field", "config": {"name": "notes", "label": "Notes", "rows": 6, "auto_resize": true}}]',
        '{"border": "var(--color-border)"}'
    ),
    (
        'file-upload',
        'input',
        'FileUpload',
        '{
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "label": { "type": "string" },
                "accept": { "type": "string", "description": "MIME type or extension filter, e.g. image/*" },
                "multiple": { "type": "boolean", "default": false },
                "max_size_mb": { "type": "number", "default": 10 },
                "drag_drop": { "type": "boolean", "default": true }
            },
            "required": ["name"]
        }',
        true,
        'File upload with drag-and-drop support.',
        '[{"context": "Avatar upload", "config": {"name": "avatar", "label": "Profile Image", "accept": "image/*", "max_size_mb": 2}}]',
        '{"border": "var(--color-border)", "drag_bg": "var(--color-surface-alt)"}'
    ),
    (
        'search-input',
        'input',
        'SearchInput',
        '{
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "placeholder": { "type": "string", "default": "Search..." },
                "debounce_ms": { "type": "integer", "default": 300 },
                "on_search": { "type": "string", "description": "Action ID called with search query" },
                "clearable": { "type": "boolean", "default": true }
            },
            "required": ["name", "on_search"]
        }',
        true,
        'Debounced search input with clear button.',
        '[{"context": "Table search", "config": {"name": "q", "on_search": "search_items", "debounce_ms": 300}}]',
        '{"icon_color": "var(--color-muted)"}'
    ),
    (
        'color-picker',
        'input',
        'ColorPicker',
        '{
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "label": { "type": "string" },
                "format": { "type": "string", "enum": ["hex", "rgb", "hsl", "oklch"], "default": "hex" },
                "presets": { "type": "array", "items": { "type": "string" } },
                "alpha": { "type": "boolean", "default": false }
            },
            "required": ["name"]
        }',
        true,
        'Color picker with format options and preset swatches.',
        '[{"context": "Brand color", "config": {"name": "brand_color", "label": "Brand Color", "format": "oklch"}}]',
        '{}'
    ),
    (
        'slider',
        'input',
        'Slider',
        '{
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "label": { "type": "string" },
                "min": { "type": "number", "default": 0 },
                "max": { "type": "number", "default": 100 },
                "step": { "type": "number", "default": 1 },
                "range": { "type": "boolean", "default": false },
                "show_value": { "type": "boolean", "default": true }
            },
            "required": ["name"]
        }',
        true,
        'Range slider with optional dual-handle range mode.',
        '[{"context": "Price range", "config": {"name": "price", "label": "Price", "min": 0, "max": 1000, "range": true}}]',
        '{"track": "var(--color-border)", "fill": "var(--color-accent)", "thumb": "var(--color-accent)"}'
    )
ON CONFLICT (slug) DO UPDATE
    SET schema = EXCLUDED.schema,
        description = EXCLUDED.description,
        usage_examples = EXCLUDED.usage_examples,
        design_tokens = EXCLUDED.design_tokens,
        updated_at = now();

-- ============================================================
-- ACTION (6)
-- ============================================================

INSERT INTO flint_a2ui.components
    (slug, category, primitive_type, schema, is_base, description, usage_examples, design_tokens)
VALUES
    (
        'button',
        'action',
        'Button',
        '{
            "type": "object",
            "properties": {
                "label": { "type": "string" },
                "action": { "type": "string", "description": "Action ID" },
                "variant": { "type": "string", "enum": ["primary", "secondary", "ghost", "destructive", "link"], "default": "primary" },
                "size": { "type": "string", "enum": ["sm", "md", "lg"], "default": "md" },
                "icon": { "type": "string" },
                "icon_position": { "type": "string", "enum": ["left", "right"], "default": "left" },
                "loading": { "type": "boolean", "default": false },
                "disabled": { "type": "boolean", "default": false }
            },
            "required": ["label", "action"]
        }',
        true,
        'Action button with variant, icon, and loading state support.',
        '[{"context": "Save button", "config": {"label": "Save", "action": "save_record", "variant": "primary"}}]',
        '{"primary_bg": "var(--color-accent)", "primary_text": "var(--color-on-accent)"}'
    ),
    (
        'action-bar',
        'action',
        'ActionBar',
        '{
            "type": "object",
            "properties": {
                "actions": { "type": "array", "items": { "type": "object", "properties": {"label": {"type": "string"}, "action": {"type": "string"}, "variant": {"type": "string"}}, "required": ["label", "action"] } },
                "position": { "type": "string", "enum": ["top", "bottom", "inline"], "default": "inline" }
            },
            "required": ["actions"]
        }',
        true,
        'Grouped action bar for form or record actions.',
        '[{"context": "Form actions", "config": {"actions": [{"label": "Save", "action": "save", "variant": "primary"}, {"label": "Cancel", "action": "cancel", "variant": "ghost"}]}}]',
        '{"gap": "var(--space-sm)"}'
    ),
    (
        'dropdown-menu',
        'action',
        'DropdownMenu',
        '{
            "type": "object",
            "properties": {
                "trigger_label": { "type": "string" },
                "trigger_icon": { "type": "string" },
                "items": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "label": { "type": "string" },
                            "action": { "type": "string" },
                            "icon": { "type": "string" },
                            "destructive": { "type": "boolean", "default": false },
                            "separator_before": { "type": "boolean", "default": false }
                        },
                        "required": ["label", "action"]
                    }
                }
            },
            "required": ["items"]
        }',
        true,
        'Trigger-activated dropdown action menu.',
        '[{"context": "Row actions", "config": {"trigger_icon": "more-horizontal", "items": [{"label": "Edit", "action": "edit"}, {"label": "Delete", "action": "delete", "destructive": true}]}}]',
        '{"bg": "var(--color-surface)", "border": "var(--color-border)"}'
    ),
    (
        'context-menu',
        'action',
        'ContextMenu',
        '{
            "type": "object",
            "properties": {
                "items": { "type": "array", "items": { "type": "object", "properties": {"label": {"type": "string"}, "action": {"type": "string"}, "shortcut": {"type": "string"}}, "required": ["label", "action"] } },
                "trigger": { "type": "string", "description": "Component slug to wrap with right-click context" }
            },
            "required": ["items"]
        }',
        true,
        'Right-click context menu for table rows and interactive elements.',
        '[{"context": "Table row context", "config": {"items": [{"label": "Copy ID", "action": "copy_id", "shortcut": "⌘C"}]}}]',
        '{"bg": "var(--color-surface)", "border": "var(--color-border)"}'
    ),
    (
        'fab',
        'action',
        'Fab',
        '{
            "type": "object",
            "properties": {
                "action": { "type": "string" },
                "icon": { "type": "string" },
                "label": { "type": "string" },
                "position": { "type": "string", "enum": ["bottom-right", "bottom-left", "bottom-center"], "default": "bottom-right" },
                "size": { "type": "string", "enum": ["sm", "md", "lg"], "default": "md" }
            },
            "required": ["action", "icon"]
        }',
        true,
        'Floating action button for primary page action.',
        '[{"context": "Create new record", "config": {"action": "create_record", "icon": "plus", "position": "bottom-right"}}]',
        '{"bg": "var(--color-accent)", "text": "var(--color-on-accent)"}'
    ),
    (
        'link',
        'action',
        'Link',
        '{
            "type": "object",
            "properties": {
                "label": { "type": "string" },
                "href": { "type": "string" },
                "action": { "type": "string" },
                "target": { "type": "string", "enum": ["_self", "_blank"], "default": "_self" },
                "variant": { "type": "string", "enum": ["default", "muted", "underline"], "default": "default" }
            },
            "required": ["label"]
        }',
        true,
        'Text link with optional action or href.',
        '[{"context": "View details", "config": {"label": "View details", "action": "view_record"}}]',
        '{"color": "var(--color-accent)", "hover": "var(--color-accent-hover)"}'
    )
ON CONFLICT (slug) DO UPDATE
    SET schema = EXCLUDED.schema,
        description = EXCLUDED.description,
        usage_examples = EXCLUDED.usage_examples,
        design_tokens = EXCLUDED.design_tokens,
        updated_at = now();

-- ============================================================
-- NAVIGATION (6)
-- ============================================================

INSERT INTO flint_a2ui.components
    (slug, category, primitive_type, schema, is_base, description, usage_examples, design_tokens)
VALUES
    (
        'nav-bar',
        'navigation',
        'NavBar',
        '{
            "type": "object",
            "properties": {
                "brand": { "type": "object", "properties": {"logo": {"type": "string"}, "name": {"type": "string"}} },
                "items": { "type": "array", "items": { "type": "object", "properties": {"label": {"type": "string"}, "route": {"type": "string"}, "icon": {"type": "string"}}, "required": ["label", "route"] } },
                "user_menu": { "type": "object" },
                "sticky": { "type": "boolean", "default": true }
            }
        }',
        true,
        'Top navigation bar with brand, nav items, and user menu.',
        '[{"context": "App nav", "config": {"brand": {"name": "Flint"}, "items": [{"label": "Dashboard", "route": "/"}], "sticky": true}}]',
        '{"bg": "var(--color-surface)", "border": "var(--color-border)", "height": "64px"}'
    ),
    (
        'sidebar',
        'navigation',
        'Sidebar',
        '{
            "type": "object",
            "properties": {
                "items": { "type": "array", "items": { "type": "object", "properties": {"label": {"type": "string"}, "route": {"type": "string"}, "icon": {"type": "string"}, "children": {"type": "array"}}, "required": ["label", "route"] } },
                "collapsed": { "type": "boolean", "default": false },
                "collapsible": { "type": "boolean", "default": true },
                "width": { "type": "string", "default": "240px" }
            }
        }',
        true,
        'Collapsible sidebar navigation with nested items.',
        '[{"context": "Admin sidebar", "config": {"items": [{"label": "Users", "route": "/users", "icon": "users"}], "collapsible": true}}]',
        '{"bg": "var(--color-surface)", "active_bg": "var(--color-surface-alt)", "width": "240px"}'
    ),
    (
        'tabs',
        'navigation',
        'Tabs',
        '{
            "type": "object",
            "properties": {
                "items": { "type": "array", "items": { "type": "object", "properties": {"label": {"type": "string"}, "value": {"type": "string"}, "icon": {"type": "string"}}, "required": ["label", "value"] } },
                "default_value": { "type": "string" },
                "variant": { "type": "string", "enum": ["line", "pill", "card"], "default": "line" },
                "content": { "type": "object", "description": "Map of value to child component" }
            },
            "required": ["items"]
        }',
        true,
        'Tabbed navigation with content panels.',
        '[{"context": "Profile tabs", "config": {"items": [{"label": "Overview", "value": "overview"}, {"label": "Settings", "value": "settings"}], "default_value": "overview"}}]',
        '{"active_border": "var(--color-accent)", "active_text": "var(--color-accent)"}'
    ),
    (
        'breadcrumb',
        'navigation',
        'Breadcrumb',
        '{
            "type": "object",
            "properties": {
                "items": { "type": "array", "items": { "type": "object", "properties": {"label": {"type": "string"}, "route": {"type": "string"}}, "required": ["label"] } },
                "separator": { "type": "string", "default": "/" },
                "max_items": { "type": "integer", "description": "Collapse middle items if exceeded" }
            },
            "required": ["items"]
        }',
        true,
        'Hierarchical breadcrumb trail.',
        '[{"context": "Page location", "config": {"items": [{"label": "Home", "route": "/"}, {"label": "Users", "route": "/users"}, {"label": "Detail"}]}}]',
        '{"separator_color": "var(--color-muted)", "active_color": "var(--color-text)"}'
    ),
    (
        'pagination',
        'navigation',
        'Pagination',
        '{
            "type": "object",
            "properties": {
                "total": { "type": "integer" },
                "page": { "type": "integer", "default": 1 },
                "page_size": { "type": "integer", "default": 20 },
                "on_page_change": { "type": "string", "description": "Action ID" },
                "show_size_picker": { "type": "boolean", "default": false },
                "size_options": { "type": "array", "items": { "type": "integer" }, "default": [10, 20, 50] }
            },
            "required": ["total", "on_page_change"]
        }',
        true,
        'Page navigation control for paginated data.',
        '[{"context": "Table pagination", "config": {"total": 500, "page": 1, "page_size": 20, "on_page_change": "go_to_page"}}]',
        '{"active_bg": "var(--color-accent)", "active_text": "var(--color-on-accent)"}'
    ),
    (
        'stepper',
        'navigation',
        'Stepper',
        '{
            "type": "object",
            "properties": {
                "steps": { "type": "array", "items": { "type": "object", "properties": {"label": {"type": "string"}, "description": {"type": "string"}}, "required": ["label"] } },
                "current_step": { "type": "integer", "default": 0 },
                "orientation": { "type": "string", "enum": ["horizontal", "vertical"], "default": "horizontal" },
                "on_step_change": { "type": "string" }
            },
            "required": ["steps"]
        }',
        true,
        'Multi-step progress indicator for wizards and onboarding.',
        '[{"context": "Setup wizard", "config": {"steps": [{"label": "Account"}, {"label": "Profile"}, {"label": "Done"}], "current_step": 0}}]',
        '{"completed_bg": "var(--color-accent)", "active_border": "var(--color-accent)"}'
    )
ON CONFLICT (slug) DO UPDATE
    SET schema = EXCLUDED.schema,
        description = EXCLUDED.description,
        usage_examples = EXCLUDED.usage_examples,
        design_tokens = EXCLUDED.design_tokens,
        updated_at = now();

-- ============================================================
-- FEEDBACK (8)
-- ============================================================

INSERT INTO flint_a2ui.components
    (slug, category, primitive_type, schema, is_base, description, usage_examples, design_tokens)
VALUES
    (
        'alert',
        'feedback',
        'Alert',
        '{
            "type": "object",
            "properties": {
                "message": { "type": "string" },
                "variant": { "type": "string", "enum": ["info", "success", "warning", "error"], "default": "info" },
                "title": { "type": "string" },
                "dismissible": { "type": "boolean", "default": false },
                "icon": { "type": "boolean", "default": true }
            },
            "required": ["message"]
        }',
        true,
        'Inline alert message with semantic variants.',
        '[{"context": "Save error", "config": {"message": "Could not save changes. Try again.", "variant": "error", "dismissible": true}}]',
        '{"info": "var(--color-info)", "success": "var(--color-success)", "warning": "var(--color-warning)", "error": "var(--color-error)"}'
    ),
    (
        'toast',
        'feedback',
        'Toast',
        '{
            "type": "object",
            "properties": {
                "message": { "type": "string" },
                "variant": { "type": "string", "enum": ["info", "success", "warning", "error"], "default": "info" },
                "duration_ms": { "type": "integer", "default": 4000 },
                "position": { "type": "string", "enum": ["top-right", "top-left", "bottom-right", "bottom-left", "top-center", "bottom-center"], "default": "bottom-right" },
                "action_label": { "type": "string" },
                "action": { "type": "string" }
            },
            "required": ["message"]
        }',
        true,
        'Transient toast notification with auto-dismiss.',
        '[{"context": "Save success", "config": {"message": "Changes saved.", "variant": "success", "duration_ms": 3000}}]',
        '{"bg": "var(--color-surface)", "border": "var(--color-border)"}'
    ),
    (
        'modal',
        'feedback',
        'Modal',
        '{
            "type": "object",
            "properties": {
                "title": { "type": "string" },
                "content": { "type": "object", "description": "Child component tree" },
                "size": { "type": "string", "enum": ["sm", "md", "lg", "xl", "full"], "default": "md" },
                "close_on_overlay": { "type": "boolean", "default": true },
                "footer_actions": { "type": "array", "items": { "type": "object" } }
            }
        }',
        true,
        'Overlay modal dialog with title, content, and footer actions.',
        '[{"context": "Confirm delete", "config": {"title": "Delete record?", "size": "sm", "footer_actions": [{"label": "Delete", "action": "confirm_delete", "variant": "destructive"}, {"label": "Cancel", "action": "close_modal", "variant": "ghost"}]}}]',
        '{"bg": "var(--color-surface)", "overlay": "rgba(0,0,0,0.5)"}'
    ),
    (
        'dialog',
        'feedback',
        'Dialog',
        '{
            "type": "object",
            "properties": {
                "title": { "type": "string" },
                "description": { "type": "string" },
                "confirm_label": { "type": "string", "default": "Confirm" },
                "cancel_label": { "type": "string", "default": "Cancel" },
                "confirm_action": { "type": "string" },
                "variant": { "type": "string", "enum": ["default", "destructive"], "default": "default" }
            },
            "required": ["title", "confirm_action"]
        }',
        true,
        'Confirmation dialog for destructive or important actions.',
        '[{"context": "Logout confirm", "config": {"title": "Sign out?", "description": "You will be logged out of all sessions.", "confirm_label": "Sign out", "confirm_action": "logout", "variant": "destructive"}}]',
        '{"bg": "var(--color-surface)"}'
    ),
    (
        'loading-spinner',
        'feedback',
        'LoadingSpinner',
        '{
            "type": "object",
            "properties": {
                "size": { "type": "string", "enum": ["sm", "md", "lg"], "default": "md" },
                "label": { "type": "string" },
                "overlay": { "type": "boolean", "default": false }
            }
        }',
        true,
        'Animated loading spinner with optional label and full-screen overlay.',
        '[{"context": "Page load", "config": {"size": "lg", "label": "Loading...", "overlay": true}}]',
        '{"color": "var(--color-accent)"}'
    ),
    (
        'progress-bar',
        'feedback',
        'ProgressBar',
        '{
            "type": "object",
            "properties": {
                "value": { "type": "number", "minimum": 0, "maximum": 100 },
                "label": { "type": "string" },
                "show_value": { "type": "boolean", "default": true },
                "variant": { "type": "string", "enum": ["default", "success", "warning", "error"], "default": "default" },
                "animated": { "type": "boolean", "default": false }
            },
            "required": ["value"]
        }',
        true,
        'Horizontal progress bar with percentage and variants.',
        '[{"context": "Upload progress", "config": {"value": 65, "label": "Uploading...", "animated": true}}]',
        '{"fill": "var(--color-accent)", "track": "var(--color-border)"}'
    ),
    (
        'empty-state',
        'feedback',
        'EmptyState',
        '{
            "type": "object",
            "properties": {
                "title": { "type": "string" },
                "description": { "type": "string" },
                "icon": { "type": "string" },
                "action": { "type": "object", "properties": {"label": {"type": "string"}, "action_id": {"type": "string"}}, "required": ["label", "action_id"] }
            },
            "required": ["title"]
        }',
        true,
        'Empty state with illustration, title, description, and optional CTA.',
        '[{"context": "No records found", "config": {"title": "No results", "description": "Try adjusting your filters.", "icon": "search", "action": {"label": "Clear filters", "action_id": "clear_filters"}}}]',
        '{"icon_color": "var(--color-muted)", "text_color": "var(--color-text)"}'
    ),
    (
        'error-boundary',
        'feedback',
        'ErrorBoundary',
        '{
            "type": "object",
            "properties": {
                "title": { "type": "string", "default": "Something went wrong" },
                "description": { "type": "string" },
                "retry_action": { "type": "string" },
                "retry_label": { "type": "string", "default": "Try again" },
                "children": { "type": "object", "description": "Protected child component" }
            }
        }',
        true,
        'Error boundary wrapper with retry action for component error states.',
        '[{"context": "Data widget error", "config": {"title": "Failed to load", "retry_action": "reload_widget"}}]',
        '{"icon_color": "var(--color-error)"}'
    )
ON CONFLICT (slug) DO UPDATE
    SET schema = EXCLUDED.schema,
        description = EXCLUDED.description,
        usage_examples = EXCLUDED.usage_examples,
        design_tokens = EXCLUDED.design_tokens,
        updated_at = now();

-- ============================================================
-- SYSTEM (1)
-- ============================================================

INSERT INTO flint_a2ui.components
    (slug, category, primitive_type, schema, is_base, description)
VALUES
    (
        'flint-meta-schema',
        'system',
        'SchemaDescriptor',
        '{ "type": "object", "description": "Flint schema metadata descriptor from flint_meta.agui_descriptor()" }',
        true,
        'System component: self-registration of the Flint database schema metadata surface. '
        'Not rendered by frontends. Used by agents to discover available tables, functions, '
        'and capabilities via the MCP/A2A protocol.'
    )
ON CONFLICT (slug) DO NOTHING;
