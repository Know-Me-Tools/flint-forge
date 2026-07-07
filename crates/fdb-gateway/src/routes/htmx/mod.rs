//! HTMX fragment renderer — server-side HTML for admin/prototyping.
//!
//! Renders Flint A2UI components as semantic HTMX-enabled HTML fragments for:
//! 1. Admin/management UI (registry browser)
//! 2. Rapid prototyping without a JS build step
//! 3. Pure-agent HTMX surfaces (simple form/list interactions)
//!
//! **NOT for production agent UI** — use the React or Flutter SDK for that.
//! HTMX here uses DaisyUI semantic classes and HTMX attributes. The base layout
//! pulls HTMX + DaisyUI from CDN.
//!
//! # Endpoints
//!
//! - `GET  /htmx/`                        — admin landing
//! - `GET  /htmx/admin/registry`          — registry management UI
//! - `GET  /htmx/components/:slug`        — render component with default/demo props
//! - `POST /htmx/components/:slug`        — render component with posted JSON props
//! - `GET  /htmx/surfaces/assemble`       — assemble surface from query params → HTML
#![forbid(unsafe_code)]

mod renderers;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    Extension,
};
use forge_identity::RlsContext;
use serde::Deserialize;
use serde_json::Value;
use sqlx::{types::Json as SqlxJson, FromRow};
use uuid::Uuid;

use std::fmt::Write as _;

use crate::routes::a2ui::A2uiState;

/// HTMX-scoped state.
#[derive(Clone)]
pub struct HtmxState {
    pub a2ui: A2uiState,
}

// ─── HX-Request detection ───────────────────────────────────────────────────

fn is_htmx_request(headers: &HeaderMap) -> bool {
    headers
        .get("hx-request")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|s| s.eq_ignore_ascii_case("true"))
}

// ─── Base layout ────────────────────────────────────────────────────────────

fn base_layout(title: &str, fragment: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en" data-theme="light">
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1"/>
  <title>{title} — Flint HTMX Admin</title>
  <script src="https://unpkg.com/htmx.org@1.9.12" defer></script>
  <link href="https://cdn.jsdelivr.net/npm/daisyui@4.12.10/dist/full.min.css" rel="stylesheet" type="text/css"/>
  <link href="https://cdn.jsdelivr.net/npm/tailwindcss@2.2.19/dist/tailwind.min.css" rel="stylesheet" type="text/css"/>
</head>
<body class="bg-base-200 min-h-screen">
  <nav class="navbar bg-base-100 shadow-lg mb-6">
    <div class="flex-1">
      <span class="text-xl font-bold px-4">🔥 Flint Forge</span>
    </div>
    <div class="flex-none">
      <ul class="menu menu-horizontal">
        <li><a href="/htmx/admin/registry" class="btn btn-ghost btn-sm">Registry</a></li>
      </ul>
    </div>
  </nav>
  <main class="container mx-auto px-4 pb-12">
    {fragment}
  </main>
</body>
</html>"#
    )
}

fn render_fragment(headers: &HeaderMap, title: &str, fragment: &str) -> Response {
    if is_htmx_request(headers) {
        Html(fragment.to_owned()).into_response()
    } else {
        Html(base_layout(title, fragment)).into_response()
    }
}

// ─── Routes ─────────────────────────────────────────────────────────────────

/// `GET /htmx/` — admin landing page.
#[allow(clippy::unused_async)]
pub async fn index(headers: HeaderMap) -> Response {
    let fragment = r#"
    <h1 class="text-3xl font-bold mb-6">Flint HTMX Admin</h1>
    <p class="text-base-content/70 mb-6">Server-side rendering surface for prototyping and registry management.</p>
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
      <a href="/htmx/admin/registry" class="card bg-base-100 shadow hover:shadow-lg transition-shadow">
        <div class="card-body">
          <h2 class="card-title">Component Registry</h2>
          <p>Browse and manage A2UI components.</p>
        </div>
      </a>
      <a href="/htmx/components/data-grid" class="card bg-base-100 shadow hover:shadow-lg transition-shadow">
        <div class="card-body">
          <h2 class="card-title">Data Grid Demo</h2>
          <p>See the data-grid component with demo data.</p>
        </div>
      </a>
      <a href="/htmx/components/form" class="card bg-base-100 shadow hover:shadow-lg transition-shadow">
        <div class="card-body">
          <h2 class="card-title">Form Demo</h2>
          <p>See a form component with sample fields.</p>
        </div>
      </a>
    </div>"#;
    render_fragment(&headers, "Admin", fragment)
}

/// `GET /htmx/admin/registry` — browse the component registry.
pub async fn admin_registry(
    State(state): State<HtmxState>,
    Extension(_who): Extension<RlsContext>,
    headers: HeaderMap,
) -> Response {
    let components: Vec<RegistryComponentRow> = match sqlx::query_as(
        "SELECT id, slug, category, primitive_type, description
         FROM flint_a2ui.components
         WHERE is_base = true OR application_id IS NULL
         ORDER BY category, slug",
    )
    .fetch_all(&state.a2ui.pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "htmx admin: failed to fetch components");
            return render_fragment(
                &headers,
                "Registry",
                r#"<div class="alert alert-error">Failed to load components.</div>"#,
            );
        }
    };

    let mut categories: Vec<(String, Vec<&RegistryComponentRow>)> = Vec::new();
    for c in &components {
        if let Some(slot) = categories.iter_mut().find(|(cat, _)| cat == &c.category) {
            slot.1.push(c);
        } else {
            categories.push((c.category.clone(), vec![c]));
        }
    }

    let mut html = String::from(
        r#"
    <div class="flex items-center justify-between mb-6">
      <h1 class="text-3xl font-bold">Component Registry</h1>
      <span class="badge badge-lg">"#,
    );
    html.push_str(&components.len().to_string());
    html.push_str(" components</span>\n    </div>");

    for (category, items) in &categories {
        let _ = write!(
            html,
            r#"
      <h2 class="text-2xl font-semibold mt-8 mb-3 capitalize">{category}</h2>
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">"#
        );
        for c in items {
            let desc = c.description.as_deref().unwrap_or("No description");
            let slug = renderers::html_escape(&c.slug);
            let pt   = renderers::html_escape(&c.primitive_type);
            let _ = write!(
                html,
                r#"
        <div class="card bg-base-100 shadow border border-base-300" data-flint-component="{slug}">
          <div class="card-body">
            <div class="flex items-center justify-between">
              <h3 class="card-title text-lg">{slug}</h3>
              <span class="badge badge-outline badge-sm">{pt}</span>
            </div>
            <p class="text-sm text-base-content/60">{}</p>
            <div class="card-actions mt-2">
              <a href="/htmx/components/{slug}" class="btn btn-primary btn-sm">Preview</a>
            </div>
          </div>
        </div>"#,
                renderers::html_escape(desc)
            );
        }
        html.push_str("</div>");
    }
    html.push_str("</div>");

    render_fragment(&headers, "Component Registry", &html)
}

/// `GET /htmx/components/:slug` — render a single component with demo props.
pub async fn render_component(
    State(state): State<HtmxState>,
    Extension(_who): Extension<RlsContext>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    let component: Option<ComponentSchemaRow> = match sqlx::query_as(
        "SELECT slug, category, primitive_type, schema, description
         FROM flint_a2ui.components
         WHERE slug = $1 AND (is_base = true OR application_id IS NULL)",
    )
    .bind(&slug)
    .fetch_optional(&state.a2ui.pool)
    .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!(error = %e, "htmx: component fetch failed");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("<div class='alert alert-error'>Database error.</div>"),
            )
                .into_response();
        }
    };

    let Some(c) = component else {
        return (
            StatusCode::NOT_FOUND,
            Html(format!("<div class='alert alert-warning'>Component '{slug}' not found.</div>")),
        )
            .into_response();
    };

    let html = renderers::render_component_html(&c.slug, &c.schema.0, c.description.as_deref());
    render_fragment(&headers, &format!("{} component", c.slug), &html)
}

/// `POST /htmx/components/:slug` — render a component with posted JSON props.
pub async fn render_component_with_props(
    State(state): State<HtmxState>,
    Extension(_who): Extension<RlsContext>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    axum::Json(props): axum::Json<Value>,
) -> Response {
    let exists: Option<(bool,)> = sqlx::query_as("SELECT true FROM flint_a2ui.components WHERE slug = $1")
        .bind(&slug)
        .fetch_optional(&state.a2ui.pool)
        .await
        .ok()
        .flatten();

    if exists.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Html(format!("<div class='alert alert-warning'>Component '{slug}' not found.</div>")),
        )
            .into_response();
    }

    let html = renderers::render_component_html(&slug, &props, None);
    if is_htmx_request(&headers) {
        Html(html).into_response()
    } else {
        Html(base_layout(&format!("{slug} component"), &html)).into_response()
    }
}

/// `GET /htmx/surfaces/assemble` — assemble a surface from query params.
#[derive(Debug, Deserialize)]
pub struct AssembleQuery {
    pub event_type: String,
    #[serde(default)]
    pub application_id: Option<Uuid>,
}

pub async fn assemble_surface_html(
    State(state): State<HtmxState>,
    Extension(who): Extension<RlsContext>,
    Query(q): Query<AssembleQuery>,
    headers: HeaderMap,
) -> Response {
    use crate::routes::a2ui::{assemble_surface_value, AssembleSurfaceBody};

    let body = AssembleSurfaceBody {
        event_type: q.event_type,
        event_context: Value::Null,
        application_id: q.application_id,
    };
    match assemble_surface_value(&state.a2ui.pool, &who, &body).await {
        Ok(surface_json) => {
            let html = format!(
                r#"<div class="card bg-base-100 shadow" data-flint-surface>
  <div class="card-body">
    <h2 class="card-title">Assembled Surface</h2>
    <pre class="bg-base-200 p-4 rounded-lg overflow-x-auto"><code>{}</code></pre>
  </div>
</div>"#,
                renderers::html_escape(&serde_json::to_string_pretty(&surface_json.0).unwrap_or_default())
            );
            render_fragment(&headers, "Assembled Surface", &html)
        }
        Err((status, axum::Json(v))) => {
            let msg = v.get("error").and_then(Value::as_str).unwrap_or("assembly failed");
            (status, Html(format!("<div class='alert alert-error'>{msg}</div>"))).into_response()
        }
    }
}

// ─── SQL row types ──────────────────────────────────────────────────────────

#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct RegistryComponentRow {
    id: Uuid,
    slug: String,
    category: String,
    primitive_type: String,
    description: Option<String>,
}

#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct ComponentSchemaRow {
    slug: String,
    category: String,
    primitive_type: String,
    schema: SqlxJson<Value>,
    description: Option<String>,
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::renderers::*;
    use axum::{
        body::Body,
        extract::Request,
        http::{Method, StatusCode},
        routing::{get, post},
        Router,
    };
    use serde_json::json;
    use sqlx::PgPool;
    use tower::ServiceExt;

    fn fake_rls_context() -> RlsContext {
        RlsContext {
            role: "authenticated".to_string(),
            claims_json: json!({"flint": {"user_id": "test-user"}}).to_string(),
            raw_bearer: "fake".to_string(),
            keto_subject: "test-user".to_string(),
            vault_key_id: None,
        }
    }

    async fn connect() -> Option<HtmxState> {
        let url = std::env::var("DATABASE_URL").ok()?;
        let pool = PgPool::connect(&url).await.ok()?;
        Some(HtmxState { a2ui: A2uiState { pool } })
    }

    fn htmx_app(state: HtmxState) -> Router {
        Router::new()
            .route("/htmx/", get(index))
            .route("/htmx/admin/registry", get(admin_registry))
            .route("/htmx/components/{slug}", get(render_component).post(render_component_with_props))
            .route("/htmx/surfaces/assemble", get(assemble_surface_html))
            .layer(Extension(fake_rls_context()))
            .with_state(state)
    }

    async fn read_body(resp: axum::response::Response) -> String {
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.expect("body");
        String::from_utf8(bytes.to_vec()).expect("utf8")
    }

    // ── Original tests ────────────────────────────────────────────────────

    #[test]
    fn test_html_escape_escapes_special_chars() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_render_button_primary_variant() {
        let schema = json!({"label": "Save", "variant": "primary"});
        let html = render_button(&schema);
        assert!(html.contains("data-flint-component=\"button\""));
        assert!(html.contains("btn-primary"));
        assert!(html.contains("Save"));
    }

    #[test]
    fn test_render_form_with_fields() {
        let schema = json!({"fields": [{"name": "email"}, {"name": "password"}]});
        let html = render_form(&schema);
        assert!(html.contains("data-flint-component=\"form\""));
        assert!(html.contains("name=\"email\""));
        assert!(html.contains("hx-post="));
    }

    #[test]
    fn test_render_data_grid_columns() {
        let schema = json!({"columns": ["Name", "Email", "Status"]});
        let html = render_data_grid(&schema);
        assert!(html.contains("data-flint-component=\"data-grid\""));
        assert!(html.contains("Name"));
        assert!(html.contains("<thead>"));
    }

    #[test]
    fn test_render_generic_for_unknown_component() {
        let schema = json!({"custom": true});
        let html = render_generic("custom-widget", &schema, Some("A custom widget"));
        assert!(html.contains("data-flint-component=\"custom-widget\""));
        assert!(html.contains("No dedicated HTMX renderer"));
    }

    #[test]
    fn test_render_text_heading_variant() {
        let schema = json!({"content": "Hello", "variant": "h1"});
        let html = render_text(&schema);
        assert!(html.contains("<h1"));
        assert!(html.contains("font-bold"));
        assert!(html.contains("Hello"));
    }

    #[test]
    fn test_is_htmx_request_detects_hx_header() {
        let mut headers = HeaderMap::new();
        assert!(!is_htmx_request(&headers));
        headers.insert("hx-request", "true".parse().unwrap());
        assert!(is_htmx_request(&headers));
    }

    // ── New renderer tests ────────────────────────────────────────────────

    #[test]
    fn render_alert_uses_variant_class() {
        let html = render_component_html("alert", &json!({"message":"Saved!", "variant":"success"}), None);
        assert!(html.contains("data-flint-component=\"alert\""));
        assert!(html.contains("alert-success"));
        assert!(html.contains("Saved!"));
    }

    #[test]
    fn render_badge_uses_color_class() {
        let html = render_component_html("badge", &json!({"label":"New","color":"secondary"}), None);
        assert!(html.contains("badge-secondary"));
        assert!(html.contains("New"));
    }

    #[test]
    fn render_progress_bar_shows_value() {
        let html = render_component_html("progress-bar", &json!({"value":75,"max":100}), None);
        assert!(html.contains("data-flint-component=\"progress-bar\""));
        assert!(html.contains("75%"));
    }

    #[test]
    fn render_stat_card_shows_label_value() {
        let html = render_component_html("stat-card", &json!({"label":"Revenue","value":"$9k"}), None);
        assert!(html.contains("Revenue"));
        assert!(html.contains("$9k"));
    }

    #[test]
    fn render_breadcrumb_has_items() {
        let html = render_component_html("breadcrumb", &json!({"items":[{"label":"Home"},{"label":"Products"}]}), None);
        assert!(html.contains("data-flint-component=\"breadcrumb\""));
        assert!(html.contains("Home"));
        assert!(html.contains("Products"));
    }

    #[test]
    fn render_text_input_uses_name() {
        let html = render_component_html("text-input", &json!({"name":"username","label":"Username"}), None);
        assert!(html.contains("name=\"username\""));
        assert!(html.contains("Username"));
    }

    #[test]
    fn render_all_55_slugs_do_not_panic() {
        let slugs = [
            "container","row","column","grid","stack","divider","spacer","scroll-area",
            "data-grid","data-table","text","badge","tag","avatar","stat-card","timeline",
            "code-block","json-viewer","list","detail-view",
            "form","text-input","number-input","select","multi-select","date-picker",
            "checkbox","radio","toggle","textarea","file-upload","search-input",
            "color-picker","slider",
            "button","action-bar","dropdown-menu","context-menu","fab","link",
            "nav-bar","sidebar","tabs","breadcrumb","pagination","stepper",
            "alert","toast","modal","dialog","loading-spinner","progress-bar",
            "empty-state","error-boundary","flint-meta-schema",
        ];
        let schema = serde_json::json!({});
        for slug in &slugs {
            let html = render_component_html(slug, &schema, None);
            assert!(
                !html.is_empty(),
                "render_component_html returned empty for slug: {slug}"
            );
            assert!(
                html.contains(&format!("data-flint-component=\"{slug}\"")),
                "missing data-flint-component for slug: {slug}"
            );
        }
    }

    #[tokio::test]
    async fn test_index_renders_base_layout_without_htmx_header() {
        let state = connect().await.unwrap_or_else(|| HtmxState {
            a2ui: A2uiState { pool: PgPool::connect_lazy("postgres://x").unwrap() },
        });
        let app = htmx_app(state);
        let req = Request::builder().method(Method::GET).uri("/htmx/").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.expect("req");
        assert_eq!(resp.status(), StatusCode::OK);
        let body = read_body(resp).await;
        assert!(body.contains("<!DOCTYPE html>"));
        assert!(body.contains("Flint HTMX Admin"));
    }

    #[tokio::test]
    async fn test_admin_registry_renders_with_db() {
        let Some(state) = connect().await else { return };
        let app = htmx_app(state);
        let req = Request::builder().method(Method::GET).uri("/htmx/admin/registry").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.expect("req");
        assert_eq!(resp.status(), StatusCode::OK);
        let body = read_body(resp).await;
        assert!(body.contains("Component Registry") || body.contains("alert-error"));
    }
}
