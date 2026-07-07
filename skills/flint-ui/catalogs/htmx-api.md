# fdb-gateway HTMX API Reference

Server-side HTML renderer for admin/prototyping surfaces. All routes require
`Authorization: Bearer <token>`. Returns HTML fragments for HTMX requests,
full pages for direct browser navigation.

---

## Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/htmx/` | Admin landing page |
| `GET` | `/htmx/admin/registry` | Component registry browser |
| `GET` | `/htmx/components/:slug` | Render component with demo props |
| `POST` | `/htmx/components/:slug` | Render component with posted JSON props |
| `GET` | `/htmx/surfaces/assemble` | Assemble + render surface as HTML |

---

## Fragment vs. Full-Page Response

All endpoints detect `HX-Request: true` and return:
- **HTMX requests**: HTML fragment only (no `<html>/<head>/<body>`)
- **Direct browser**: Full page wrapped in base layout (HTMX + DaisyUI CDN)

```html
<!-- HTMX trigger (returns fragment) -->
<div hx-get="/htmx/components/data-grid"
     hx-headers='{"Authorization": "Bearer {{token}}"}'
     hx-trigger="load"
     hx-swap="outerHTML"></div>

<!-- Full page (returns complete HTML) -->
<a href="/htmx/admin/registry">Registry</a>
```

---

## POST /htmx/components/:slug

Post JSON props to render a component with specific data:

```html
<form hx-post="/htmx/components/form"
      hx-ext="json-enc"
      hx-headers='{"Authorization": "Bearer {{token}}"}'
      hx-target="#preview"
      hx-swap="outerHTML">
  <textarea name="props">
    {
      "fields": [
        {"name": "email", "type": "email", "label": "Email"},
        {"name": "role",  "type": "text",  "label": "Role"}
      ]
    }
  </textarea>
  <button type="submit">Preview</button>
</form>
<div id="preview"></div>
```

---

## GET /htmx/surfaces/assemble?event_type=...

Assemble an A2UI surface from a database event and render as HTML:

```
GET /htmx/surfaces/assemble?event_type=record.select&application_id=<uuid>
Authorization: Bearer <token>
HX-Request: true
```

---

## Component Renderers

The server includes dedicated HTML renderers for 7 common components.
All others fall back to a JSON-inspect card.

| Slug | Renderer | Notes |
|---|---|---|
| `data-grid` | ✅ Custom | DaisyUI table with zebra rows |
| `form` | ✅ Custom | HTMX-wired, hx-post to REST |
| `button` | ✅ Custom | DaisyUI btn variants |
| `text` | ✅ Custom | h1–h3, body, caption |
| `card` | ✅ Custom | DaisyUI card |
| `tabs` | ✅ Custom | DaisyUI tab-lifted |
| All others | Generic | Shows schema as JSON in `<pre>` |

---

## HTML Attributes Convention

All rendered components carry a `data-flint-component="<slug>"` attribute
for JavaScript and CSS targeting:

```html
<div data-flint-component="data-grid" class="overflow-x-auto">
  <table class="table table-zebra w-full">...</table>
</div>

<form data-flint-component="form"
      hx-post="/api/public/example"
      hx-target="#form-result"
      hx-swap="innerHTML">
  ...
</form>
```

---

## AG-UI SSE → HTMX

For real-time agent UIs, combine AG-UI SSE with HTMX SSE extension:

```html
<!-- Subscribe to a run's event stream -->
<div hx-ext="sse"
     sse-connect="/agents/v1/run-abc123/events"
     sse-swap="TextMessageContent"
     hx-headers='{"Authorization": "Bearer {{token}}"}'>
  <div id="agent-output"></div>
</div>
```

Custom event types (e.g. `a2ui:surface`) can trigger htmx OOB swaps
by matching on the SSE event name.

---

## Design System

All HTMX fragments use DaisyUI (v4) semantic classes loaded from CDN:

```
btn btn-primary / btn-secondary / btn-outline / btn-ghost
card card-body card-title card-actions
table table-zebra
badge badge-primary / badge-outline
alert alert-info / alert-success / alert-warning / alert-error
form-control label label-text input input-bordered
tabs tab tab-lifted tab-active
```
