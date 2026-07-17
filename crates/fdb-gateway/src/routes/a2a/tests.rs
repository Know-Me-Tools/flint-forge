use super::types::{INVALID_PARAMS, TASK_NOT_FOUND};
use super::*;
use axum::{
    body::Body,
    extract::Request,
    http::{Method, StatusCode},
    routing::{get, post},
    Extension, Router,
};
use forge_identity::RlsContext;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

use crate::routes::a2ui::A2uiState;

fn fake_rls_context(user_id: &str) -> RlsContext {
    RlsContext {
        role: "authenticated".to_string(),
        claims_json: json!({"flint": {"user_id": user_id}}).to_string(),
        raw_bearer: "fake".to_string(),
        keto_subject: user_id.to_string(),
        vault_key_id: None,
    }
}

async fn connect() -> Option<A2aState> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let pool = PgPool::connect(&url).await.ok()?;
    Some(A2aState {
        a2ui: A2uiState { pool },
    })
}

fn a2a_app(state: A2aState, user_id: &str) -> Router {
    Router::new()
        .route("/.well-known/agent.json", get(agent_card))
        .route("/a2a/v1", post(handle_a2a))
        .layer(Extension(fake_rls_context(user_id)))
        .with_state(state)
}

/// Agent card has no state dependency, so test it standalone.
fn agent_card_app() -> Router {
    Router::new().route("/.well-known/agent.json", get(agent_card))
}

async fn read_json(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("body");
    serde_json::from_slice(&bytes).expect("valid json")
}

fn rpc(id: i64, method: &str, params: &Value) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    }))
    .unwrap()
}

#[tokio::test]
async fn test_agent_card_has_three_skills() {
    let app = agent_card_app();
    let req = Request::builder()
        .uri("/.well-known/agent.json")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.expect("req");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_json(resp).await;
    assert_eq!(body["name"], "flint-a2ui-registry");
    let skills = body["skills"].as_array().expect("skills array");
    assert_eq!(skills.len(), 3);
    let ids: Vec<&str> = skills.iter().filter_map(|s| s["id"].as_str()).collect();
    assert!(ids.contains(&"a2ui.component.discover"));
    assert!(ids.contains(&"a2ui.component.assemble"));
    assert!(ids.contains(&"a2ui.search.semantic"));
}

#[tokio::test]
async fn test_tasks_list_returns_three_tasks() {
    let Some(state) = connect().await else { return };
    let app = a2a_app(state, "anonymous-user");
    let req = Request::builder()
        .method(Method::POST)
        .uri("/a2a/v1")
        .header("content-type", "application/json")
        .body(Body::from(rpc(1, "tasks/list", &json!({}))))
        .unwrap();
    let resp = app.oneshot(req).await.expect("req");
    let body = read_json(resp).await;
    let tasks = body["result"]["tasks"].as_array().expect("tasks array");
    assert_eq!(tasks.len(), 3);
}

#[tokio::test]
async fn test_component_discover_via_query() {
    let Some(state) = connect().await else { return };
    let app = a2a_app(state, "anonymous-user");
    let req = Request::builder()
        .method(Method::POST)
        .uri("/a2a/v1")
        .header("content-type", "application/json")
        .body(Body::from(rpc(
            2,
            "tasks/send",
            &json!({
                "task": {
                    "id": "t-001",
                    "name": "a2ui.component.discover",
                    "input": { "query": "button" }
                }
            }),
        )))
        .unwrap();
    let resp = app.oneshot(req).await.expect("req");
    let body = read_json(resp).await;
    assert_eq!(body["result"]["task"]["state"], "completed");
    assert_eq!(body["result"]["task"]["name"], "a2ui.component.discover");
    // components array exists (may be empty if no DB match)
    assert!(body["result"]["task"]["output"]["components"].is_array());
}

#[tokio::test]
async fn test_unknown_task_returns_task_not_found() {
    let Some(state) = connect().await else { return };
    let app = a2a_app(state, "anonymous-user");
    let req = Request::builder()
        .method(Method::POST)
        .uri("/a2a/v1")
        .header("content-type", "application/json")
        .body(Body::from(rpc(
            3,
            "tasks/send",
            &json!({
                "task": { "name": "nonexistent.task", "input": {} }
            }),
        )))
        .unwrap();
    let resp = app.oneshot(req).await.expect("req");
    let body = read_json(resp).await;
    assert_eq!(body["error"]["code"], TASK_NOT_FOUND);
}

#[tokio::test]
async fn test_assemble_missing_event_type_returns_invalid_params() {
    let Some(state) = connect().await else { return };
    let app = a2a_app(state, "anonymous-user");
    let req = Request::builder()
        .method(Method::POST)
        .uri("/a2a/v1")
        .header("content-type", "application/json")
        .body(Body::from(rpc(
            4,
            "tasks/send",
            &json!({
                "task": { "name": "a2ui.component.assemble", "input": {} }
            }),
        )))
        .unwrap();
    let resp = app.oneshot(req).await.expect("req");
    let body = read_json(resp).await;
    assert_eq!(body["error"]["code"], INVALID_PARAMS);
}
