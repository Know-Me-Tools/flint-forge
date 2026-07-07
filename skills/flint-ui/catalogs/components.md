# Flint A2UI Component Catalog

All 55 base components. Each entry: slug, category, primitive_type, key props, description.

---

## LAYOUT (8)

### `container` — Container
Wraps page content with configurable max-width and padding.
```json
{ "max_width": "1280px", "padding": "var(--space-md)", "centered": true, "children": [] }
```

### `row` — Row
Horizontal flex container.
```json
{ "gap": "var(--space-sm)", "align": "center", "justify": "start", "wrap": false, "children": [] }
```

### `column` — Column
Vertical flex container.
```json
{ "gap": "var(--space-sm)", "align": "stretch", "children": [] }
```

### `grid` — Grid
CSS grid with configurable columns.
```json
{ "columns": 3, "gap": "var(--space-md)", "children": [] }
```

### `stack` — Stack
Layered children (z-axis stacking).
```json
{ "children": [] }
```

### `divider` — Divider
Horizontal rule separator.
```json
{ "orientation": "horizontal", "variant": "solid" }
```

### `spacer` — Spacer
Empty space block.
```json
{ "size": "var(--space-md)" }
```

### `scroll-area` — ScrollArea
Scrollable container with optional max-height.
```json
{ "max_height": "400px", "children": [] }
```

---

## DATA DISPLAY (12)

### `data-grid` — DataGrid
Sortable, paginated data table with column definitions.
```json
{
  "columns": [
    { "name": "id", "type": "uuid", "sortable": true },
    { "name": "status", "type": "text", "sortable": true },
    { "name": "total", "type": "number", "sortable": true }
  ],
  "data": [],
  "pagination": { "pageSize": 25 },
  "onRowClick": null
}
```

### `data-table` — DataTable
Simple table with header/rows, no built-in pagination.
```json
{ "headers": [], "rows": [], "striped": true }
```

### `text` — Text
Rendered text block with variant support.
```json
{ "content": "", "variant": "body", "truncate": false }
```
`variant`: `h1` `h2` `h3` `body` `caption` `code` `label`

### `badge` — Badge
Small status indicator.
```json
{ "label": "", "color": "blue", "size": "sm" }
```

### `tag` — Tag
Removable label chip.
```json
{ "label": "", "dismissible": false, "color": "gray" }
```

### `avatar` — Avatar
User avatar with fallback initials.
```json
{ "src": null, "name": "User", "size": "md" }
```

### `stat-card` — StatCard
Metric display with label, value, and optional delta.
```json
{ "label": "Total Revenue", "value": "$12,400", "delta": "+8%", "trend": "up" }
```

### `timeline` — Timeline
Ordered event list with timestamps.
```json
{ "events": [{ "timestamp": "", "label": "", "description": "" }] }
```

### `code-block` — CodeBlock
Syntax-highlighted code display.
```json
{ "code": "", "language": "typescript", "copyable": true, "filename": null }
```

### `json-viewer` — JsonViewer
Collapsible JSON tree explorer.
```json
{ "data": {}, "expanded": true, "max_depth": 3 }
```

### `list` — List
Ordered or unordered list of items.
```json
{ "items": [], "ordered": false, "dense": false }
```

### `detail-view` — DetailView
Label/value pair grid for record detail.
```json
{ "fields": [{ "label": "Status", "value": "active" }], "columns": 2 }
```

---

## INPUT (14)

### `form` — Form
HTMX-ready form container with field list.
```json
{
  "fields": [{ "name": "email", "type": "email", "label": "Email", "required": true }],
  "submit_label": "Submit",
  "action": "/api/public/users"
}
```

### `text-input` — TextInput
Single-line text field.
```json
{ "name": "", "label": "", "placeholder": "", "required": false, "type": "text" }
```

### `number-input` — NumberInput
Numeric field with min/max/step.
```json
{ "name": "", "label": "", "min": null, "max": null, "step": 1 }
```

### `select` — Select
Single-choice dropdown.
```json
{ "name": "", "label": "", "options": [{ "value": "", "label": "" }], "required": false }
```

### `multi-select` — MultiSelect
Multi-choice dropdown with chips.
```json
{ "name": "", "label": "", "options": [], "max_selections": null }
```

### `date-picker` — DatePicker
Calendar date selector.
```json
{ "name": "", "label": "", "format": "YYYY-MM-DD", "range": false }
```

### `checkbox` — Checkbox
Boolean toggle with label.
```json
{ "name": "", "label": "", "checked": false }
```

### `radio` — Radio
Single-choice radio group.
```json
{ "name": "", "label": "", "options": [{ "value": "", "label": "" }] }
```

### `toggle` — Toggle
On/off switch.
```json
{ "name": "", "label": "", "checked": false, "size": "md" }
```

### `textarea` — Textarea
Multi-line text field.
```json
{ "name": "", "label": "", "rows": 4, "resize": "vertical" }
```

### `file-upload` — FileUpload
Drag-and-drop file input.
```json
{ "name": "", "label": "", "accept": "*/*", "multiple": false, "max_size_mb": 10 }
```

### `search-input` — SearchInput
Debounced search field with clear button.
```json
{ "name": "", "placeholder": "Search...", "debounce_ms": 300 }
```

### `color-picker` — ColorPicker
HSL/hex color selector.
```json
{ "name": "", "label": "", "format": "hex", "value": "#000000" }
```

### `slider` — Slider
Range slider.
```json
{ "name": "", "label": "", "min": 0, "max": 100, "step": 1, "value": 50 }
```

---

## ACTION (6)

### `button` — Button
Clickable action element.
```json
{ "label": "Submit", "variant": "primary", "size": "md", "disabled": false, "loading": false }
```
`variant`: `primary` `secondary` `outline` `ghost` `destructive`

### `action-bar` — ActionBar
Row of action buttons with optional overflow menu.
```json
{ "actions": [{ "label": "", "variant": "primary", "icon": null }], "overflow_threshold": 3 }
```

### `dropdown-menu` — DropdownMenu
Triggered dropdown with menu items.
```json
{ "trigger_label": "Options", "items": [{ "label": "", "action": "" }] }
```

### `context-menu` — ContextMenu
Right-click / long-press menu.
```json
{ "items": [{ "label": "", "action": "", "destructive": false }] }
```

### `fab` — Fab
Floating action button.
```json
{ "label": "Add", "icon": "plus", "position": "bottom-right" }
```

### `link` — Link
Navigable anchor element.
```json
{ "href": "", "label": "", "external": false, "variant": "default" }
```

---

## NAVIGATION (6)

### `nav-bar` — NavBar
Top navigation bar with logo and links.
```json
{ "logo": null, "links": [{ "href": "", "label": "" }], "actions": [] }
```

### `sidebar` — Sidebar
Collapsible side navigation.
```json
{ "items": [{ "href": "", "label": "", "icon": null, "children": [] }], "collapsed": false }
```

### `tabs` — Tabs
Tab panel with content slots.
```json
{ "tabs": [{ "label": "", "content": null }], "default_tab": 0 }
```

### `breadcrumb` — Breadcrumb
Hierarchical location indicator.
```json
{ "items": [{ "href": "", "label": "" }] }
```

### `pagination` — Pagination
Page navigation control.
```json
{ "total": 100, "page": 1, "page_size": 25, "show_page_size_selector": true }
```

### `stepper` — Stepper
Multi-step process indicator.
```json
{ "steps": [{ "label": "", "description": "" }], "current": 0, "orientation": "horizontal" }
```

---

## FEEDBACK (8)

### `alert` — Alert
Inline notification banner.
```json
{ "message": "", "variant": "info", "dismissible": true }
```
`variant`: `info` `success` `warning` `error`

### `toast` — Toast
Transient notification.
```json
{ "message": "", "variant": "success", "duration_ms": 3000, "position": "top-right" }
```

### `modal` — Modal
Full-screen overlay dialog.
```json
{ "title": "", "children": [], "size": "md", "closable": true }
```

### `dialog` — Dialog
Confirmation or prompt dialog.
```json
{ "title": "", "message": "", "confirm_label": "OK", "cancel_label": "Cancel" }
```

### `loading-spinner` — LoadingSpinner
Animated loading indicator.
```json
{ "size": "md", "label": null }
```

### `progress-bar` — ProgressBar
Determinate or indeterminate progress.
```json
{ "value": 60, "max": 100, "indeterminate": false, "label": null }
```

### `empty-state` — EmptyState
Placeholder when no data is available.
```json
{ "title": "No items", "description": null, "action": null, "illustration": null }
```

### `error-boundary` — ErrorBoundary
Error fallback UI.
```json
{ "message": "Something went wrong.", "retry_label": "Try again" }
```

---

## SYSTEM (1)

### `flint-meta-schema` — FlintMetaSchema
Internal system component used by `flint_meta` introspection. Not rendered directly.
```json
{ "schema_name": "", "table_name": "" }
```
