//! Component previews use the same catalog lookup and Axum handlers as the mounted HTMX routes.
use super::{htmx_app, read_body, A2uiState, HtmxState};
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

async fn preview(app: &Router, slug: &str, props: Value, fragment: bool) -> String {
    let mut request = Request::builder()
        .method(Method::POST)
        .uri(format!("/htmx/components/{slug}"))
        .header("content-type", "application/json");
    if fragment {
        request = request.header("hx-request", "true");
    }
    let response = app
        .clone()
        .oneshot(
            request
                .body(Body::from(props.to_string()))
                .expect("preview request"),
        )
        .await
        .expect("router");
    assert_eq!(response.status(), StatusCode::OK, "{slug}");
    let html = read_body(response).await;
    assert_eq!(html.contains("<!DOCTYPE html>"), !fragment, "{slug}");
    assert!(
        html.contains(&format!("data-flint-component=\"{slug}\"")),
        "{slug}"
    );
    html
}

#[tokio::test]
async fn registered_components_render_supplied_semantic_variants_and_feedback_states() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        return;
    };
    let pool = PgPool::connect(&url).await.expect("seeded test database");
    let app = htmx_app(HtmxState {
        a2ui: A2uiState { pool },
    });
    for (variant, element) in [("h2", "<h2"), ("h3", "<h3"), ("caption", "text-sm")] {
        let html = preview(
            &app,
            "text",
            json!({"content":"Publication","variant":variant}),
            false,
        )
        .await;
        assert!(html.contains(element) && html.contains("Publication"));
    }
    for (slug, props, expected) in [
        (
            "button",
            json!({"label":"Cancel","variant":"ghost"}),
            "btn-ghost",
        ),
        (
            "button",
            json!({"label":"Back","variant":"secondary"}),
            "btn-secondary",
        ),
        (
            "button",
            json!({"label":"Preview","variant":"outline"}),
            "btn-outline",
        ),
        (
            "tag",
            json!({"label":"Draft","dismissible":true}),
            "btn-circle",
        ),
        (
            "badge",
            json!({"label":"Draft","color":"warning"}),
            "badge-warning",
        ),
        ("avatar", json!({"name":"Ada Lin","size":"sm"}), ">AL<"),
        (
            "stat-card",
            json!({"label":"Queue","value":"12","delta":"+2","trend":"up"}),
            "text-success",
        ),
        (
            "toast",
            json!({"message":"Updating","variant":"info"}),
            "alert-info",
        ),
        (
            "loading-spinner",
            json!({"size":"lg","label":"Loading results"}),
            "Loading results",
        ),
        (
            "empty-state",
            json!({"action":{"label":"Create a document"}}),
            "Create a document",
        ),
    ] {
        let html = preview(&app, slug, props, true).await;
        assert!(html.contains(expected), "{slug}: {html}");
    }
    let progress = preview(&app, "progress-bar", json!({"indeterminate":true}), true).await;
    assert!(progress.contains("<progress") && !progress.contains("value="));
}

#[tokio::test]
async fn catalog_outage_returns_registry_feedback_and_failed_preview_status() {
    let pool = PgPool::connect_lazy("postgres://localhost/unopened").expect("lazy pool");
    pool.close().await;
    let app = htmx_app(HtmxState {
        a2ui: A2uiState { pool },
    });
    for (path, status, message) in [
        (
            "/htmx/admin/registry",
            StatusCode::OK,
            "Failed to load components.",
        ),
        (
            "/htmx/components/button",
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error.",
        ),
    ] {
        let request = Request::builder()
            .uri(path)
            .header("hx-request", "true")
            .body(Body::empty())
            .expect("request");
        let response = app.clone().oneshot(request).await.expect("router");
        assert_eq!(response.status(), status);
        let html = read_body(response).await;
        assert!(html.contains("alert-error") && html.contains(message));
        assert!(!html.contains("<!DOCTYPE html>"));
    }
}
