# p5-c012 — Flint HTMX Renderer (Axum + Askama)

**Phase:** 5 — Flint A2UI Component Registry  
**Priority:** P3 (prototyping / admin surface — lower priority than React/Flutter SDKs)  
**Depends on:** p5-c002 (base components seed), p5-c006 (REST API)  
**Blocks:** p5-c013 (OpenDesign integration — needs HTML fragments for design preview)

---

## What this change delivers

A server-side HTML fragment renderer in `fdb-gateway` that renders Flint A2UI components as semantic HTMX-enabled HTML via Askama templates. Intended for:

1. **Admin/management UI** — server-rendered Rust admin panel for `flint_a2ui` registry management
2. **Prototyping** — rapid iteration on component layouts without a JS build step
3. **Pure-agent HTMX surfaces** — agents that generate HTMX fragments for simple form/list interactions
4. **OpenDesign ideation** — HTML fragments for design exploration and DESIGN.md previews

**NOT intended for**: production agent-generated UI surfaces (use React or Flutter SDKs for those — HTMX is too tightly coupled to HTML presentation layer per research in `.firecrawl/htmx-axum-agent-ui-2026.md`).

---

## Architecture

### Stack (Research-Validated)

- **Axum** — HTTP server (already in `fdb-gateway`)
- **Askama** — type-safe Jinja2-like templates (`askama` crate)
- **HTMX** — client-side reactivity via `hx-*` attributes
- **axum-htmx** — HX-* header parsing/response helpers
- **DaisyUI** — Tailwind CSS component layer (no JS, semantic classes)

### File Structure (additions to `fdb-gateway`)

```
crates/fdb-gateway/
├── src/
│   ├── routes/
│   │   └── htmx.rs             # HTMX fragment route handlers
│   └── templates/
│       ├── base.html            # Askama base layout (includes HTMX + DaisyUI CDN)
│       ├── components/
│       │   ├── data_grid.html   # DataGrid HTMX fragment
│       │   ├── form.html        # Form HTMX fragment
│       │   ├── card.html        # Card HTMX fragment
│       │   ├── agent_chat.html  # AgentChat with SSE streaming
│       │   ├── tool_call.html   # ToolCall card fragment
│       │   └── ...              # One template per Flint component slug
│       └── admin/
│           ├── registry.html    # Registry management UI
│           └── applications.html
```

### Route Structure

```
GET  /htmx/components/:slug              # Render component with default props (demo)
POST /htmx/components/:slug              # Render component with POSTed props JSON
GET  /htmx/surfaces/assemble             # Assemble surface from query params, return HTML
GET  /htmx/admin/registry               # Registry management page
GET  /htmx/admin/components/:id/edit    # Edit component form fragment
SSE  /htmx/stream/:surface_id           # AG-UI SSE stream → HTMX OOB swaps
```

### Axum Handler Pattern

```rust
// crates/fdb-gateway/src/routes/htmx.rs

#[derive(Template)]
#[template(path = "components/data_grid.html")]
struct DataGridTemplate {
    columns: Vec<ColumnDef>,
    rows: Vec<serde_json::Value>,
    pagination: PaginationConfig,
    surface_id: String,
}

async fn htmx_data_grid(
    HxRequest(is_htmx): HxRequest,
    Extension(pool): Extension<deadpool_postgres::Pool>,
    Json(props): Json<DataGridProps>,
) -> impl IntoResponse {
    // Execute query against props.data_source with RLS context
    let rows = execute_data_source_query(&pool, &props).await?;
    let template = DataGridTemplate::from(props, rows);
    // Return full page if not HTMX request, fragment if HTMX
    if is_htmx {
        HtmlTemplate(template).into_response()
    } else {
        HtmlTemplate(BaseTemplate { content: template }).into_response()
    }
}
```

### HTMX SSE Streaming Pattern

```html
<!-- templates/components/agent_chat.html -->
<div id="agent-chat-{{ surface_id }}"
     hx-ext="sse"
     sse-connect="/htmx/stream/{{ surface_id }}"
     class="flint-agent-chat"
     data-flint-component="agent-chat">
  <div id="messages-{{ surface_id }}">
    {% for msg in messages %}
      <div class="message {{ msg.role }}" sse-swap="message:{{ surface_id }}">
        {{ msg.content }}
      </div>
    {% endfor %}
  </div>
  <div id="streaming-{{ surface_id }}" sse-swap="stream:{{ surface_id }}"></div>
</div>
```

### Axum SSE Handler

```rust
async fn htmx_stream(
    Path(surface_id): Path<String>,
    Extension(ag_ui_state): Extension<AgUiSurfaceState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = ag_ui_state.subscribe(surface_id).map(|event| {
        let html_fragment = render_ag_ui_event_as_html(&event);
        Ok(Event::default()
            .event(event.event_type())
            .data(html_fragment))
    });
    Sse::new(stream)
}
```

### Component Template Pattern

Each component template uses DaisyUI semantic classes and HTMX attributes:

```html
<!-- templates/components/form.html (Askama) -->
<form id="form-{{ surface_id }}"
      data-flint-component="form"
      hx-post="/api/{{ table_schema }}/{{ table_name }}"
      hx-target="#form-result-{{ surface_id }}"
      hx-swap="outerHTML">
  {% for field in fields %}
    <div class="form-control mb-4">
      <label class="label" for="{{ field.name }}-{{ surface_id }}">
        <span class="label-text">{{ field.label }}</span>
      </label>
      {% include "components/input/{{ field.component }}.html" %}
    </div>
  {% endfor %}
  <button type="submit" class="btn btn-primary">{{ submit_label }}</button>
</form>
<div id="form-result-{{ surface_id }}"></div>
```

---

## Gate Tests

- [ ] `GET /htmx/components/data-grid` returns valid HTML with DaisyUI classes and `data-flint-component="data-grid"`
- [ ] `POST /htmx/components/form` with JSON props returns populated form fragment
- [ ] SSE endpoint sends HTMX fragments as AG-UI events arrive
- [ ] `GET /htmx/admin/registry` renders registry management UI listing all base components
- [ ] All templates pass W3C HTML validation
- [ ] No JavaScript in rendered HTML (pure HTMX + DaisyUI)
- [ ] Full page renders for non-HTMX requests (baseline usability without JS)
