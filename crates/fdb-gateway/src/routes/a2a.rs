//! A2A (Agent-to-Agent) protocol surface — task handler registry.
//!
//! Exposes three A2UI tasks via the A2A JSON-RPC protocol so agents built on
//! the A2A spec (Google A2A v0.1+) can discover and assemble Flint components.
//!
//! # Endpoints
//!
//! - `GET  /.well-known/agent.json` — Agent Card describing capabilities + skills
//! - `POST /a2a/v1` — JSON-RPC 2.0 dispatch (`tasks/send`, `tasks/list`)
//!
//! # Security
//!
//! Mounted behind `rls_layer::require_rls`; every task call runs under the
//! caller's verified `RlsContext`. Tasks delegate to the same inner functions
//! as the REST + MCP surfaces — single SQL authority.
#![forbid(unsafe_code)]

use axum::{
    extract::State,
    response::Json,
    Extension,
};
use forge_identity::RlsContext;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::routes::a2ui::{
    self, AssembleSurfaceBody, A2uiState, ListComponentsQuery, SearchComponentsBody,
};

/// A2A server-scoped state. Wraps the A2UI route state so task handlers can
/// call the shared inner functions.
#[derive(Clone)]
pub struct A2aState {
    pub a2ui: A2uiState,
}

// ─── A2A protocol types ─────────────────────────────────────────────────────

/// JSON-RPC 2.0 request envelope (same shape as MCP).
#[derive(Debug, Deserialize)]
pub struct A2aRequest {
    #[allow(dead_code)]
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

/// A2A error object.
#[derive(Debug, Serialize)]
pub struct A2aError {
    pub code: i32,
    pub message: String,
}

impl A2aError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }
}

/// A2A task states (subset of the spec's TaskState enum).
#[allow(dead_code)]
pub enum TaskState {
    Submitted,
    Working,
    InputRequired,
    Completed,
    Canceled,
    Failed,
}

impl TaskState {
    fn as_str(&self) -> &'static str {
        match self {
            TaskState::Submitted => "submitted",
            TaskState::Working => "working",
            TaskState::InputRequired => "input-required",
            TaskState::Completed => "completed",
            TaskState::Canceled => "canceled",
            TaskState::Failed => "failed",
        }
    }
}

#[allow(dead_code)]
const PARSE_ERROR: i32 = -32700;
const METHOD_NOT_FOUND: i32 = -32601;
const INVALID_PARAMS: i32 = -32602;
const INTERNAL_ERROR: i32 = -32603;
const TASK_NOT_FOUND: i32 = -32001;

// ─── Agent Card ─────────────────────────────────────────────────────────────

/// `GET /.well-known/agent.json` — Agent Card per the A2A spec.
///
/// Describes the Flint A2UI Registry agent, its capabilities, and the three
/// skills it exposes. Agents use this to discover what tasks they can delegate.
pub async fn agent_card() -> Json<Value> {
    Json(json!({
        "name": "flint-a2ui-registry",
        "description": "Flint Forge A2UI Component Registry — discovers and assembles UI components for LLM agents.",
        "version": env!("CARGO_PKG_VERSION"),
        "protocolVersion": "0.1",
        "url": "/a2a/v1",
        "capabilities": {
            "taskPush": true,
            "taskPull": false,
            "streaming": false
        },
        "skills": [
            {
                "id": "a2ui.component.discover",
                "name": "Discover UI Component",
                "description": "Find a UI component by natural language description. Returns matching components with slug, category, and description.",
                "inputSchema": {
                    "type": "object",
                    "required": ["query"],
                    "properties": {
                        "query":      { "type": "string", "description": "Natural language description of the desired component" },
                        "limit":      { "type": "integer", "minimum": 1, "maximum": 50, "default": 10 },
                        "app_id":     { "type": "string", "format": "uuid" }
                    }
                }
            },
            {
                "id": "a2ui.component.assemble",
                "name": "Assemble A2UI Surface",
                "description": "Assemble an A2UI surface from an event context. Applies application-specific assembly rules and falls back to default table bindings.",
                "inputSchema": {
                    "type": "object",
                    "required": ["event_type"],
                    "properties": {
                        "event_type":     { "type": "string", "description": "Event name driving the assembly (e.g. 'mount', 'record.select')" },
                        "event_payload":  { "type": "object", "description": "Event payload (table, record id, etc.)" },
                        "application_id": { "type": "string", "format": "uuid" }
                    }
                }
            },
            {
                "id": "a2ui.search.semantic",
                "name": "Semantic Search Components",
                "description": "Semantic vector search for UI components using hybrid text + embedding similarity.",
                "inputSchema": {
                    "type": "object",
                    "required": ["query"],
                    "properties": {
                        "query":    { "type": "string", "description": "Natural language description of the desired component" },
                        "limit":    { "type": "integer", "minimum": 1, "maximum": 50, "default": 10 },
                        "app_id":   { "type": "string", "format": "uuid" }
                    }
                }
            }
        ]
    }))
}

// ─── JSON-RPC dispatch ──────────────────────────────────────────────────────

/// `POST /a2a/v1` — A2A JSON-RPC 2.0 dispatch.
pub async fn handle_a2a(
    State(state): State<A2aState>,
    Extension(who): Extension<RlsContext>,
    Json(req): Json<A2aRequest>,
) -> Json<Value> {
    let result = dispatch(&state, &who, &req.method, req.params.as_ref()).await;
    Json(match result {
        Ok(value) => json!({ "jsonrpc": "2.0", "id": req.id, "result": value }),
        Err(e) => json!({
            "jsonrpc": "2.0",
            "id": req.id,
            "error": { "code": e.code, "message": e.message }
        }),
    })
}

async fn dispatch(
    state: &A2aState,
    who: &RlsContext,
    method: &str,
    params: Option<&Value>,
) -> Result<Value, A2aError> {
    match method {
        "tasks/list" => Ok(json!({ "tasks": task_list() })),
        "tasks/send" => {
            let task_name = params
                .and_then(|p| p.get("task"))
                .and_then(|t| t.get("name"))
                .and_then(Value::as_str)
                .ok_or_else(|| A2aError::new(INVALID_PARAMS, "task.name required"))?;
            let task_input = params
                .and_then(|p| p.get("task"))
                .and_then(|t| t.get("input"))
                .unwrap_or(&Value::Null);
            let task_id = params
                .and_then(|p| p.get("task"))
                .and_then(|t| t.get("id"))
                .and_then(Value::as_str)
                .map_or_else(|| Uuid::new_v4().to_string(), str::to_owned);
            dispatch_task(state, who, &task_id, task_name, task_input).await
        }
        _ => Err(A2aError::new(
            METHOD_NOT_FOUND,
            format!("unknown method: {method}"),
        )),
    }
}

/// Return the list of supported task definitions.
fn task_list() -> Vec<Value> {
    vec![
        json!({
            "name": "a2ui.component.discover",
            "description": "Find a UI component by natural language description",
        }),
        json!({
            "name": "a2ui.component.assemble",
            "description": "Assemble an A2UI surface from an event context",
        }),
        json!({
            "name": "a2ui.search.semantic",
            "description": "Semantic vector search for UI components",
        }),
    ]
}

// ─── Task dispatch ──────────────────────────────────────────────────────────

async fn dispatch_task(
    state: &A2aState,
    who: &RlsContext,
    task_id: &str,
    name: &str,
    input: &Value,
) -> Result<Value, A2aError> {
    let output = match name {
        "a2ui.component.discover" => task_component_discover(state, who, input).await?,
        "a2ui.component.assemble" => task_component_assemble(state, who, input).await?,
        "a2ui.search.semantic" => task_search_semantic(state, who, input).await?,
        other => {
            return Err(A2aError::new(
                TASK_NOT_FOUND,
                format!("unknown task: {other}"),
            ));
        }
    };
    // Wrap the output in the A2A Task envelope per the spec.
    Ok(json!({
        "task": {
            "id": task_id,
            "name": name,
            "state": TaskState::Completed.as_str(),
            "output": output,
        }
    }))
}

// ─── Task handlers (delegate to A2UI inner functions) ───────────────────────

async fn task_component_discover(
    state: &A2aState,
    who: &RlsContext,
    input: &Value,
) -> Result<Value, A2aError> {
    // Component discovery uses list_components + optional semantic search.
    // When `query` is provided, semantic search is preferred; otherwise list.
    if let Some(query) = input.get("query").and_then(Value::as_str) {
        let limit = input
            .get("limit")
            .and_then(Value::as_i64)
            .map_or(10, |i| i32::try_from(i).unwrap_or(10));
        let app_id = parse_uuid_opt(input, "app_id")?;
        let body = SearchComponentsBody {
            query: query.to_owned(),
            limit,
            app_id,
        };
        let result = a2ui::search_components_value(&state.a2ui.pool, who, &body)
            .await
            .map_err(http_to_a2a_error)?;
        // Adapt the REST shape to the A2A task output schema.
        let components = result
            .0
            .get("results")
            .cloned()
            .unwrap_or(Value::Array(vec![]));
        Ok(json!({ "components": components }))
    } else {
        let app_id = parse_uuid_opt(input, "app_id")?;
        let query = ListComponentsQuery { app_id, category: None };
        let result = a2ui::list_components_value(&state.a2ui.pool, who, &query)
            .await
            .map_err(http_to_a2a_error)?;
        let components = result
            .0
            .get("components")
            .cloned()
            .unwrap_or(Value::Array(vec![]));
        Ok(json!({ "components": components }))
    }
}

async fn task_component_assemble(
    state: &A2aState,
    who: &RlsContext,
    input: &Value,
) -> Result<Value, A2aError> {
    let event_type = input
        .get("event_type")
        .and_then(Value::as_str)
        .ok_or_else(|| A2aError::new(INVALID_PARAMS, "event_type required"))?
        .to_owned();
    let event_payload = input
        .get("event_payload")
        .cloned()
        .unwrap_or(Value::Null);
    let application_id = parse_uuid_opt(input, "application_id")?;
    let body = AssembleSurfaceBody {
        event_type,
        event_context: event_payload,
        application_id,
    };
    let surface = a2ui::assemble_surface_value(&state.a2ui.pool, who, &body)
        .await
        .map_err(http_to_a2a_error)?;
    Ok(json!({ "surface": surface.0 }))
}

async fn task_search_semantic(
    state: &A2aState,
    who: &RlsContext,
    input: &Value,
) -> Result<Value, A2aError> {
    let query = input
        .get("query")
        .and_then(Value::as_str)
        .ok_or_else(|| A2aError::new(INVALID_PARAMS, "query required"))?
        .to_owned();
    let limit = input
        .get("limit")
        .and_then(Value::as_i64)
        .map_or(10, |i| i32::try_from(i).unwrap_or(10));
    let app_id = parse_uuid_opt(input, "app_id")?;
    let body = SearchComponentsBody { query, limit, app_id };
    let result = a2ui::search_components_value(&state.a2ui.pool, who, &body)
        .await
        .map_err(http_to_a2a_error)?;
    let results = result
        .0
        .get("results")
        .cloned()
        .unwrap_or(Value::Array(vec![]));
    Ok(json!({ "results": results }))
}

// ─── helpers ────────────────────────────────────────────────────────────────

/// Parse an optional uuid from the JSON args under `key`.
fn parse_uuid_opt(input: &Value, key: &str) -> Result<Option<Uuid>, A2aError> {
    match input.get(key).and_then(Value::as_str) {
        None => Ok(None),
        Some(s) => Uuid::parse_str(s)
            .map(Some)
            .map_err(|_| A2aError::new(INVALID_PARAMS, format!("invalid {key}"))),
    }
}

/// Convert the REST error tuple into an A2A error.
fn http_to_a2a_error(err: (axum::http::StatusCode, Json<Value>)) -> A2aError {
    let (status, Json(v)) = err;
    let msg = v
        .get("error")
        .and_then(Value::as_str)
        .map_or_else(|| format!("HTTP {}", status.as_u16()), str::to_owned);
    A2aError::new(INTERNAL_ERROR, msg)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
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

    async fn connect() -> Option<A2aState> {
        let url = std::env::var("DATABASE_URL").ok()?;
        let pool = PgPool::connect(&url).await.ok()?;
        Some(A2aState { a2ui: A2uiState { pool } })
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
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.expect("body");
        serde_json::from_slice(&bytes).expect("valid json")
    }

    fn rpc(id: i64, method: &str, params: Value) -> Vec<u8> {
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
            .body(Body::from(rpc(1, "tasks/list", json!({}))))
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
            .body(Body::from(rpc(2, "tasks/send", json!({
                "task": {
                    "id": "t-001",
                    "name": "a2ui.component.discover",
                    "input": { "query": "button" }
                }
            }))))
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
            .body(Body::from(rpc(3, "tasks/send", json!({
                "task": { "name": "nonexistent.task", "input": {} }
            }))))
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
            .body(Body::from(rpc(4, "tasks/send", json!({
                "task": { "name": "a2ui.component.assemble", "input": {} }
            }))))
            .unwrap();
        let resp = app.oneshot(req).await.expect("req");
        let body = read_json(resp).await;
        assert_eq!(body["error"]["code"], INVALID_PARAMS);
    }
}
