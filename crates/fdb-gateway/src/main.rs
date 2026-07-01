//! Flint Quarry composition root: REST, GraphQL (Q/M/Sub), /rpc, /healthz.
#![forbid(unsafe_code)]

mod keto_sync;

use std::sync::Arc;

use async_graphql_axum::{GraphQLProtocol, GraphQLWebSocket};
use axum::{
    Router,
    extract::{State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::get,
};
use serde::Deserialize;
use serde_json::json;

use fdb_app::graphql::introspection::{IntrospectionMerger, is_introspection_query};
use fdb_auth::rls_from_bearer;
use fdb_domain::{GraphQlRequest, VectorRpcRequest};
use fdb_ports::GraphQlExecutor;
use fdb_postgres::{PgGraphQl, PgVectorRpc};
use fdb_reflection::{ReflectionEngine, StateManager};
use sqlx::PgPool;

/// Shared gateway state — available in all route handlers via `State<GatewayState>`.
#[derive(Clone)]
struct GatewayState {
    state_manager: Arc<StateManager>,
    graphql_executor: Arc<PgGraphQl>,
    vector_rpc: Arc<PgVectorRpc>,
    /// Keto relation-check adapter backed by the background sync cache.
    /// Injected into `Quarry::with_keto()` when mutation use-cases are wired,
    /// or called directly by handlers via `State<GatewayState>`.
    //
    // Scaffold-stage concession: the field is not yet read by any handler.
    // CRUD mutation handlers (p3-c014) will call `state.keto.as_ref()`.
    #[allow(dead_code)]
    keto: Option<Arc<dyn fdb_ports::KetoCheck>>,
}

/// GraphQL request body as sent by clients (queries and mutations only — subscriptions
/// use the WebSocket path).
#[derive(Debug, Deserialize)]
struct GraphQlBody {
    query: String,
    #[serde(default)]
    variables: serde_json::Value,
    #[serde(rename = "operationName")]
    operation_name: Option<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        tracing::warn!("DATABASE_URL not set; reflection will use placeholder");
        "postgres://localhost/flint".into()
    });

    let pool = PgPool::connect(&database_url)
        .await
        .expect("reflection pool connect");

    // Apply SQL migrations (migrations/ at workspace root) on every startup.
    // sqlx::migrate! embeds the files at compile time; the migrator is idempotent —
    // already-applied migrations are skipped. Startup is aborted if any migration fails.
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("database migration failed");
    tracing::info!("database migrations applied");

    // Seed base A2UI component catalog (idempotent — ON CONFLICT DO UPDATE/DO NOTHING).
    // The seed SQL lives at scripts/seed_a2ui_components.sql relative to workspace root.
    let seed_sql = include_str!("../../../scripts/seed_a2ui_components.sql");
    sqlx::raw_sql(seed_sql)
        .execute(&pool)
        .await
        .expect("a2ui component seed failed");
    tracing::info!("a2ui base component catalog seeded");

    // Build the pg_graphql executor from the same connection string.
    let pg_pool = {
        let mut cfg = deadpool_postgres::Config::new();
        cfg.url = Some(database_url.clone());
        cfg.create_pool(
            Some(deadpool_postgres::Runtime::Tokio1),
            tokio_postgres::NoTls,
        )
        .expect("graphql pool create")
    };
    let graphql_executor = Arc::new(PgGraphQl::new(pg_pool));

    // Build the vector RPC executor from a separate pool (shares RLS context via acquire()).
    let vector_rpc_pool = {
        let mut cfg = deadpool_postgres::Config::new();
        cfg.url = Some(database_url.clone());
        cfg.create_pool(
            Some(deadpool_postgres::Runtime::Tokio1),
            tokio_postgres::NoTls,
        )
        .expect("vector-rpc pool create")
    };
    let vector_rpc = Arc::new(PgVectorRpc::new(vector_rpc_pool));

    let engine = ReflectionEngine::new(pool.clone());
    let state_manager = Arc::new(
        StateManager::new(engine, pool.clone(), database_url.clone())
            .await
            .expect("initial schema compile"),
    );

    let _listener_handle = Arc::clone(&state_manager).start_listener();

    // Spawn the Keto sync background task.
    // SECURITY: pool is the privileged reflection pool — MUST NOT be the user RLS pool.
    let keto_sync_cfg = keto_sync::keto_sync_config_from_env(Arc::new(
        PgPool::connect(&database_url)
            .await
            .expect("keto-sync pool connect"),
    ));
    let (keto_task, keto_cache) = keto_sync::KetoSyncTask::new(keto_sync_cfg);
    let _keto_sync_handle = keto_task.spawn();

    // Build the KetoCheck adapter from the sync cache. This is the
    // composition-time bridge between the background-synced cache and the
    // application layer. When Quarry is constructed, pass this via `.with_keto()`.
    let keto_adapter: Arc<dyn fdb_ports::KetoCheck> =
        Arc::new(keto_sync::KetoCacheAdapter::new(keto_cache));

    let gateway_state = GatewayState {
        state_manager: Arc::clone(&state_manager),
        graphql_executor,
        vector_rpc,
        keto: Some(keto_adapter),
    };

    // Build gateway routes as Router<()> by applying state, then merge the
    // reflection-compiled router (also Router<()>) into it. The reflection
    // router exposes CRUD (/public/<table>) and RPC (/rpc/public/<fn>) routes
    // generated from the live DatabaseModel.
    //
    // Route hot-reload note: the reflection router is mounted once at startup.
    // Handler bodies read from RestState (captured at compile time); DDL-driven
    // route-set changes require a catch-all delegate pattern (future enhancement).
    let reflection_router = state_manager
        .current()
        .router
        .as_ref()
        .clone();

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/openapi.json", get(openapi_handler))
        .route("/rpc/vector", axum::routing::post(rpc_vector_handler))
        .route(
            "/graphql",
            get(graphql_ws_handler).post(handle_graphql_query),
        )
        .with_state(gateway_state)
        .merge(reflection_router);

    let addr = "0.0.0.0:8080";
    tracing::info!(%addr, "flint-quarry listening");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}

async fn healthz(State(state): State<GatewayState>) -> Json<serde_json::Value> {
    let compiled = state.state_manager.current();
    Json(json!({
        "status": "ok",
        "service": "flint-quarry",
        "schema_version": compiled.version,
    }))
}

/// GET /openapi.json — serve the compiled OpenAPI 3.1.0 document.
///
/// No authentication required — OpenAPI docs are public, same pattern as
/// Supabase's `/rest/v1/` OpenAPI endpoint. The document is pre-compiled by
/// `OpenApiCompiler::compile()` during `StateManager::do_compile()` and hot-swapped
/// on DDL changes. Callers always receive the schema consistent with the live
/// database state at the time of the request.
async fn openapi_handler(State(state): State<GatewayState>) -> Json<serde_json::Value> {
    let compiled = state.state_manager.current();
    Json(compiled.openapi_doc.clone())
}

/// POST /rpc/vector — vector similarity search via pgvector `<=>` operator.
///
/// Accepts a `VectorRpcRequest` body, executes under the full 6-GUC RLS context,
/// and returns a JSON array of rows ordered by ascending cosine distance.
async fn rpc_vector_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(body): Json<VectorRpcRequest>,
) -> impl IntoResponse {
    let Some(bearer) = extract_bearer(&headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "missing Authorization header"})),
        )
            .into_response();
    };
    let rls = match rls_from_bearer(&bearer).await {
        Ok(ctx) => ctx,
        Err(e) => {
            tracing::warn!(error = %e, "bearer verification failed for /rpc/vector");
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "invalid or expired token"})),
            )
                .into_response();
        }
    };
    match state.vector_rpc.execute_similarity(&body, &rls).await {
        Ok(rows) => Json(rows).into_response(),
        Err(fdb_ports::BackendError::Query(msg)) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": msg})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "vector similarity error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal server error"})),
            )
                .into_response()
        }
    }
}

/// GET /graphql — WebSocket upgrade for GraphQL subscriptions (graphql-transport-ws).
///
/// Reads the current compiled `subscription_schema` from `StateManager` on each
/// connection. The schema is hot-swapped on DDL changes; each new WS connection
/// gets the schema that was current at upgrade time.
///
/// Returns `503 Service Unavailable` when no subscription schema is available
/// (schema compile failed on startup or after DDL change).
async fn graphql_ws_handler(
    State(state): State<GatewayState>,
    protocol: GraphQLProtocol,
    ws: WebSocketUpgrade,
) -> Response {
    let compiled = state.state_manager.current();
    let Some(schema) = compiled.subscription_schema.clone() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "subscription schema not available"})),
        )
            .into_response();
    };

    ws.protocols(async_graphql::http::ALL_WEBSOCKET_PROTOCOLS)
        .on_upgrade(move |socket| async move {
            GraphQLWebSocket::new(socket, schema, protocol).serve().await;
        })
}

/// POST /graphql — GraphQL query and mutation handler.
///
/// Extracts the bearer token from the `Authorization` header, builds `RlsContext`,
/// and delegates to `graphql.resolve()` via `PgGraphQl::execute()`.
/// The response is the raw pg_graphql JSON — no envelope added.
async fn handle_graphql_query(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(body): Json<GraphQlBody>,
) -> impl IntoResponse {
    let Some(bearer) = extract_bearer(&headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"errors": [{"message": "missing Authorization header"}]})),
        )
            .into_response();
    };

    let rls = match rls_from_bearer(&bearer).await {
        Ok(ctx) => ctx,
        Err(e) => {
            tracing::warn!(error = %e, "bearer verification failed");
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"errors": [{"message": "invalid or expired token"}]})),
            )
                .into_response();
        }
    };

    let req = GraphQlRequest {
        query: body.query,
        variables: body.variables,
        operation_name: body.operation_name,
    };

    let is_introspection = is_introspection_query(&req.query);

    match state.graphql_executor.execute(req, &rls).await {
        Ok(mut result) => {
            // Merge subscription types into introspection responses.
            if is_introspection {
                let compiled = state.state_manager.current();
                if let Some(sub_schema) = compiled.subscription_schema.as_ref() {
                    result = IntrospectionMerger::merge(result, sub_schema);
                }
            }
            Json(result).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "graphql execution error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"errors": [{"message": "internal server error"}]})),
            )
                .into_response()
        }
    }
}

/// Extract the raw bearer token from the `Authorization: Bearer <token>` header.
/// Returns `None` if the header is absent or malformed.
fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(ToOwned::to_owned)
}
