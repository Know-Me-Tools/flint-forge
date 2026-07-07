# p8-c004 — HTMX Remaining 48 Component Renderers

**Phase:** 8 — SDK Completeness
**Priority:** P1
**Depends on:** none (but `htmx.rs` must be split into directory module first)

## What this change delivers

Dedicated HTML renderers for the 48 slugs not yet covered by `routes/htmx.rs`.
After this change every slug in the catalog renders meaningful DaisyUI HTML
rather than the generic JSON-inspect card.

## Design

### Module split (mandatory before adding renderers)

`crates/fdb-gateway/src/routes/htmx.rs` (already ~500 lines) must be split into:

```
crates/fdb-gateway/src/routes/htmx/
├── mod.rs            ← pub handlers + render_component_html dispatch
├── renderers.rs      ← existing 7 + new renderers
└── admin.rs          ← admin_registry handler
```

### Renderer additions by category

Each renderer follows the same pattern as `render_button`, `render_form`, etc.
Grouped for parallel implementation:

**Input (13 new):** `text-input`, `number-input`, `select`, `multi-select`, `date-picker`,
`checkbox`, `radio`, `toggle`, `textarea`, `file-upload`, `search-input`, `color-picker`, `slider`

**Action (5 new):** `action-bar`, `dropdown-menu`, `context-menu`, `fab`, `link`

**Navigation (5 new):** `nav-bar`, `sidebar`, `breadcrumb`, `pagination`, `stepper`

**Feedback (8 new):** `alert`, `toast`, `modal`, `dialog`, `loading-spinner`,
`progress-bar`, `empty-state`, `error-boundary`

**Data-display (10 new):** `data-table`, `badge`, `tag`, `avatar`, `stat-card`,
`timeline`, `code-block`, `json-viewer`, `list`, `detail-view`

**Layout (7 new):** `container`, `row`, `column`, `grid`, `stack`, `divider`, `spacer`, `scroll-area`
(minus `data-grid`/`data-table` already done = 6 net new layout renderers)

### All renderers use `data-flint-component="<slug>"` attribute + DaisyUI classes
