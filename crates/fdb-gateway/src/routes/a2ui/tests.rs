use super::*;
use axum::{
    body::Body,
    extract::Request,
    http::{Method, StatusCode},
    routing::{get, post},
    Extension, Router,
};
use forge_identity::RlsContext;
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

async fn connect() -> Option<(PgPool, A2uiState)> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let pool = PgPool::connect(&url).await.ok()?;
    let state = A2uiState { pool: pool.clone() };
    Some((pool, state))
}

fn fake_rls_context(user_id: &str) -> RlsContext {
    RlsContext {
        role: "authenticated".to_string(),
        claims_json: json!({"flint": {"user_id": user_id}}).to_string(),
        raw_bearer: "fake".to_string(),
        keto_subject: user_id.to_string(),
        vault_key_id: None,
    }
}

fn a2ui_app(state: A2uiState, user_id: &str) -> Router {
    Router::new()
        .route("/a2ui/v1/components", get(list_components))
        .route("/a2ui/v1/components/search", post(search_components))
        .route(
            "/a2ui/v1/components/bindings/{schema}/{table}",
            get(get_bindings),
        )
        .route("/a2ui/v1/components/{slug}", get(get_component))
        .route("/a2ui/v1/applications", get(list_applications))
        .route("/a2ui/v1/applications/{id}", get(get_application))
        .route("/a2ui/v1/catalog/{*catalog_id}", get(get_catalog))
        .route("/a2ui/v1/surfaces/assemble", post(assemble_surface))
        .route(
            "/a2ui/v1/design-systems/{id}/tokens",
            get(get_design_system_tokens),
        )
        .layer(Extension(fake_rls_context(user_id)))
        .with_state(state)
}

async fn read_json_body(resp: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("read body");
    serde_json::from_slice(&bytes).expect("body is valid JSON")
}

#[tokio::test]
async fn test_list_components_returns_base_components() {
    let Some((_pool, state)) = connect().await else {
        return;
    };
    let app = a2ui_app(state, "anonymous-user");

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/a2ui/v1/components")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request");

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_json_body(resp).await;
    let components = body["components"].as_array().expect("components array");
    let slugs: Vec<String> = components
        .iter()
        .filter_map(|c| c["slug"].as_str().map(String::from))
        .collect();
    assert!(slugs.contains(&"data-grid".to_string()));
    assert!(slugs.contains(&"button".to_string()));
}

#[tokio::test]
async fn test_get_component_returns_schema() {
    let Some((_pool, state)) = connect().await else {
        return;
    };
    let app = a2ui_app(state, "anonymous-user");

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/a2ui/v1/components/button")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request");

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_json_body(resp).await;
    assert_eq!(body["component"]["slug"], "button");
    assert!(body["component"]["schema"].is_object());
}

#[tokio::test]
async fn test_search_components_finds_button() {
    let Some((_pool, state)) = connect().await else {
        return;
    };
    let app = a2ui_app(state, "anonymous-user");

    let req = Request::builder()
        .method(Method::POST)
        .uri("/a2ui/v1/components/search")
        .header("content-type", "application/json")
        .body(Body::from(json!({"query": "button"}).to_string()))
        .unwrap();

    let resp = app.oneshot(req).await.expect("request");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_json_body(resp).await;
    let results = body["results"].as_array().expect("results array");
    assert!(
        !results.is_empty(),
        "search should return at least one result"
    );
}

#[tokio::test]
async fn test_get_catalog_returns_json_schema() {
    let Some((_pool, state)) = connect().await else {
        return;
    };
    let app = a2ui_app(state, "anonymous-user");

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/a2ui/v1/catalog/flint-base/1.0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request");

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_json_body(resp).await;
    assert_eq!(body["$schema"], "https://a2ui.org/schemas/catalog/v0.9.1");
    assert!(body["definitions"]["Button"].is_object() || body["definitions"]["button"].is_object());
}

#[tokio::test]
async fn test_assemble_surface_validates_input() {
    let Some((_pool, state)) = connect().await else {
        return;
    };
    let app = a2ui_app(state, "anonymous-user");

    let req = Request::builder()
        .method(Method::POST)
        .uri("/a2ui/v1/surfaces/assemble")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&AssembleSurfaceBody {
                event_type: "mount".to_string(),
                event_context: json!({}),
                application_id: None,
            })
            .unwrap(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.expect("request");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_get_design_system_tokens_not_found() {
    let Some((_pool, state)) = connect().await else {
        return;
    };
    let app = a2ui_app(state, "anonymous-user");
    let id = Uuid::new_v4();
    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/a2ui/v1/design-systems/{id}/tokens"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("req");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
