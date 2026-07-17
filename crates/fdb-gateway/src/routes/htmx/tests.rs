use super::renderers::*;
use super::*;
use axum::{
    body::Body,
    extract::Request,
    http::{Method, StatusCode},
    routing::get,
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
    Some(HtmxState {
        a2ui: A2uiState { pool },
    })
}

fn htmx_app(state: HtmxState) -> Router {
    Router::new()
        .route("/htmx/", get(index))
        .route("/htmx/admin/registry", get(admin_registry))
        .route(
            "/htmx/components/{slug}",
            get(render_component).post(render_component_with_props),
        )
        .route("/htmx/surfaces/assemble", get(assemble_surface_html))
        .layer(Extension(fake_rls_context()))
        .with_state(state)
}

async fn read_body(resp: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("body");
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
    let html = render_component_html(
        "alert",
        &json!({"message":"Saved!", "variant":"success"}),
        None,
    );
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
    let html = render_component_html(
        "breadcrumb",
        &json!({"items":[{"label":"Home"},{"label":"Products"}]}),
        None,
    );
    assert!(html.contains("data-flint-component=\"breadcrumb\""));
    assert!(html.contains("Home"));
    assert!(html.contains("Products"));
}

#[test]
fn render_text_input_uses_name() {
    let html = render_component_html(
        "text-input",
        &json!({"name":"username","label":"Username"}),
        None,
    );
    assert!(html.contains("name=\"username\""));
    assert!(html.contains("Username"));
}

#[test]
fn render_all_55_slugs_do_not_panic() {
    let slugs = [
        "container",
        "row",
        "column",
        "grid",
        "stack",
        "divider",
        "spacer",
        "scroll-area",
        "data-grid",
        "data-table",
        "text",
        "badge",
        "tag",
        "avatar",
        "stat-card",
        "timeline",
        "code-block",
        "json-viewer",
        "list",
        "detail-view",
        "form",
        "text-input",
        "number-input",
        "select",
        "multi-select",
        "date-picker",
        "checkbox",
        "radio",
        "toggle",
        "textarea",
        "file-upload",
        "search-input",
        "color-picker",
        "slider",
        "button",
        "action-bar",
        "dropdown-menu",
        "context-menu",
        "fab",
        "link",
        "nav-bar",
        "sidebar",
        "tabs",
        "breadcrumb",
        "pagination",
        "stepper",
        "alert",
        "toast",
        "modal",
        "dialog",
        "loading-spinner",
        "progress-bar",
        "empty-state",
        "error-boundary",
        "flint-meta-schema",
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
        a2ui: A2uiState {
            pool: PgPool::connect_lazy("postgres://x").unwrap(),
        },
    });
    let app = htmx_app(state);
    let req = Request::builder()
        .method(Method::GET)
        .uri("/htmx/")
        .body(Body::empty())
        .unwrap();
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
    let req = Request::builder()
        .method(Method::GET)
        .uri("/htmx/admin/registry")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.expect("req");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_body(resp).await;
    assert!(body.contains("Component Registry") || body.contains("alert-error"));
}
