//! Integration tests for the MCP JSON-RPC dispatch layer.

use super::protocol::METHOD_NOT_FOUND;
use super::*;
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

fn fake_rls_context(user_id: &str) -> RlsContext {
    RlsContext {
        role: "authenticated".to_string(),
        claims_json: json!({"flint": {"user_id": user_id}}).to_string(),
        raw_bearer: "fake".to_string(),
        keto_subject: user_id.to_string(),
        vault_key_id: None,
    }
}

async fn connect() -> Option<McpState> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let pool = PgPool::connect(&url).await.ok()?;
    Some(McpState {
        a2ui: A2uiState { pool },
    })
}

fn mcp_app(state: McpState, user_id: &str) -> Router {
    Router::new()
        .route("/mcp/v1/a2ui", post(handle_mcp))
        .route("/mcp/v1/a2ui/health", get(health))
        .layer(Extension(fake_rls_context(user_id)))
        .with_state(state)
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
async fn test_initialize_returns_server_info() {
    let Some(state) = connect().await else { return };
    let app = mcp_app(state, "anonymous-user");
    let req = Request::builder()
        .method(Method::POST)
        .uri("/mcp/v1/a2ui")
        .header("content-type", "application/json")
        .body(Body::from(rpc(1, "initialize", &json!({}))))
        .unwrap();
    let resp = app.oneshot(req).await.expect("req");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_json(resp).await;
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 1);
    assert_eq!(body["result"]["serverInfo"]["name"], "flint-a2ui-registry");
    assert!(body["result"]["capabilities"]["tools"].is_object());
}

#[tokio::test]
async fn test_tools_list_returns_seven_a2ui_tools() {
    let Some(state) = connect().await else { return };
    let app = mcp_app(state, "anonymous-user");
    let req = Request::builder()
        .method(Method::POST)
        .uri("/mcp/v1/a2ui")
        .header("content-type", "application/json")
        .body(Body::from(rpc(2, "tools/list", &json!({}))))
        .unwrap();
    let resp = app.oneshot(req).await.expect("req");
    let body = read_json(resp).await;
    let tools = body["result"]["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 7, "exactly 7 a2ui tools");
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert!(names.contains(&"a2ui_list_components"));
    assert!(names.contains(&"a2ui_assemble_surface"));
}

#[tokio::test]
async fn test_a2ui_list_components_tool_returns_components() {
    let Some(state) = connect().await else { return };
    let app = mcp_app(state, "anonymous-user");
    let req = Request::builder()
        .method(Method::POST)
        .uri("/mcp/v1/a2ui")
        .header("content-type", "application/json")
        .body(Body::from(rpc(
            3,
            "tools/call",
            &json!({
                "name": "a2ui_list_components",
                "arguments": {}
            }),
        )))
        .unwrap();
    let resp = app.oneshot(req).await.expect("req");
    let body = read_json(resp).await;
    assert!(body["result"]["content"].is_array());
    let text = body["result"]["content"][0]["text"].as_str().expect("text");
    let payload: Value = serde_json::from_str(text).expect("inner json");
    let components = payload["components"].as_array().expect("components array");
    assert!(!components.is_empty(), "base components should be listed");
}

#[tokio::test]
async fn test_unknown_method_returns_method_not_found() {
    let Some(state) = connect().await else { return };
    let app = mcp_app(state, "anonymous-user");
    let req = Request::builder()
        .method(Method::POST)
        .uri("/mcp/v1/a2ui")
        .header("content-type", "application/json")
        .body(Body::from(rpc(4, "resources/read", &json!({}))))
        .unwrap();
    let resp = app.oneshot(req).await.expect("req");
    let body = read_json(resp).await;
    assert_eq!(body["error"]["code"], METHOD_NOT_FOUND);
}

#[tokio::test]
async fn test_health_endpoint() {
    let Some(state) = connect().await else { return };
    let app = mcp_app(state, "anonymous-user");
    let req = Request::builder()
        .uri("/mcp/v1/a2ui/health")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.expect("req");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["service"], "flint-a2ui-mcp");
}

#[tokio::test]
async fn test_a2ui_get_component_tool_returns_schema() {
    let Some(state) = connect().await else { return };
    let app = mcp_app(state, "anonymous-user");
    let req = Request::builder()
        .method(Method::POST)
        .uri("/mcp/v1/a2ui")
        .header("content-type", "application/json")
        .body(Body::from(rpc(
            5,
            "tools/call",
            &json!({
                "name": "a2ui_get_component",
                "arguments": { "slug": "button" }
            }),
        )))
        .unwrap();
    let resp = app.oneshot(req).await.expect("req");
    let body = read_json(resp).await;
    let text = body["result"]["content"][0]["text"].as_str().expect("text");
    let payload: Value = serde_json::from_str(text).expect("inner json");
    assert_eq!(payload["component"]["slug"], "button");
}
