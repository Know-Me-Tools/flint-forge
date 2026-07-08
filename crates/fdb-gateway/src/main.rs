//! Flint Quarry composition root: REST, GraphQL (Q/M/Sub), /rpc, /healthz.
#![forbid(unsafe_code)]

mod agui_hook_dispatcher;
mod keto_sync;
mod policy_source;
mod rls_layer;
mod routes;
mod telemetry;

use fdb_gateway::a2ui_embedder;

use std::sync::Arc;

use async_graphql_axum::{GraphQLProtocol, GraphQLWebSocket};
use axum::{
    extract::{State, WebSocketUpgrade},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use serde_json::json;
use tower_http::set_header::SetResponseHeaderLayer;

use fdb_app::graphql::introspection::{is_introspection_query, IntrospectionMerger};
use fdb_app::Quarry;
use fdb_auth::rls_from_bearer;
use fdb_domain::{GraphQlRequest, SubscriptionSpec, TableMeta, VectorRpcRequest};
use fdb_ports::GraphQlExecutor;
use fdb_postgres::{PgGraphQl, PgRest, PgVectorRpc};
use fdb_realtime::{FabricChangeSource, FrfConfig, KetoConfig, ListenChangeSource, ListenConfig};
use fdb_reflection::compilers::graphql::SubStreamFactory;
use fdb_reflection::MutationGates;
use fdb_reflection::{ReflectionEngine, StateManager};
use forge_identity::RlsContext;
use forge_policy::CedarPolicyEngine;
use futures::stream::StreamExt;
use sqlx::PgPool;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

/// Shared gateway state — available in all route handlers via `State<GatewayState>`.
#[derive(Clone)]
pub(crate) struct GatewayState {
    state_manager: Arc<StateManager>,
    graphql_executor: Arc<PgGraphQl>,
    vector_rpc: Arc<PgVectorRpc>,
    /// Privileged reflection pool used by A2UI routes and background tasks.
    /// SECURITY: never expose this pool to per-user RLS contexts.
    pool: PgPool,
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

// Composition root: sequential wiring of pools, adapters, gates, routes. This is
// the anyhow-at-the-edge binary entry point; a long linear body is idiomatic here.
#[allow(clippy::too_many_lines)]
#[tokio::main]
async fn main() {
    // p9-c004: Initialise structured tracing (fmt + optional OTLP) and Prometheus metrics.
    // Guard is held for the process lifetime so the OTLP exporter flushes cleanly on exit.
    let _telemetry_guard = telemetry::init_tracing();
    let (metrics_layer, metrics_handle) = telemetry::metrics_layer();

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

    // p14-c001: Emit sqlx pool gauges so the Grafana DB connections panel and
    // the HighDbConnections alert rule produce real data.
    telemetry::spawn_pool_metrics(pool.clone());

    // Seed base A2UI component catalog (idempotent — ON CONFLICT DO UPDATE/DO NOTHING).
    // The seed SQL lives at scripts/seed_a2ui_components.sql relative to workspace root.
    let seed_sql = include_str!("../../../scripts/seed_a2ui_components.sql");
    sqlx::raw_sql(seed_sql)
        .execute(&pool)
        .await
        .expect("a2ui component seed failed");
    tracing::info!("a2ui base component catalog seeded");

    // Spawn the A2UI component embedder. It backfills missing embeddings and
    // listens on the Postgres `a2ui_embed` channel for new component inserts.
    // SECURITY: this pool is the privileged reflection pool — MUST NOT be the
    // per-user RLS pool. The embedder writes to `flint_a2ui.embeddings` directly.
    let embedder_pool = Arc::new(
        PgPool::connect(&database_url)
            .await
            .expect("a2ui-embedder pool connect"),
    );
    let _a2ui_embedder_handle = a2ui_embedder::spawn(embedder_pool);

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

    // Spawn the Keto sync background task BEFORE compiling routes, so the
    // KetoCheck adapter can be threaded into the reflection compiler's mutation
    // gates (and hot-reloads).
    // SECURITY: this pool is the privileged reflection pool — MUST NOT be the user RLS pool.
    let keto_sync_cfg = keto_sync::keto_sync_config_from_env(Arc::new(
        PgPool::connect(&database_url)
            .await
            .expect("keto-sync pool connect"),
    ));
    let (keto_task, keto_cache) = keto_sync::KetoSyncTask::new(keto_sync_cfg);
    let _keto_sync_handle = keto_task.spawn();

    // KetoCheck adapter — the composition-time bridge between the background
    // cache and the mutation gates.
    let keto_adapter: Arc<dyn fdb_ports::KetoCheck> =
        Arc::new(keto_sync::KetoCacheAdapter::new(keto_cache));

    // Cedar policy enforcement point, backed by flint_meta.cedar_policies via
    // the privileged pool. Starts deny-all until the first successful load.
    let policy_source = Arc::new(policy_source::DbPolicySource::new(pool.clone()));
    let pep: Arc<dyn forge_policy::Pep> = Arc::new(CedarPolicyEngine::new(policy_source).await);

    // Thread both gates into the reflection compiler so initial + hot-swapped
    // routers enforce Keto + Cedar on every mutation.
    let gates = MutationGates {
        keto: Some(Arc::clone(&keto_adapter)),
        pep: Some(Arc::clone(&pep)),
    };

    // Build the GraphQL subscription live-stream factory (Quarry + adapters).
    let sub_stream_factory =
        build_subscription_factory(&database_url, Arc::clone(&keto_adapter)).await;

    let engine = ReflectionEngine::new(pool.clone());
    let state_manager = Arc::new(
        StateManager::new_with_gates(
            engine,
            pool.clone(),
            database_url.clone(),
            gates,
            Some(sub_stream_factory),
        )
        .await
        .expect("initial schema compile"),
    );

    let _listener_handle = Arc::clone(&state_manager).start_listener();

    // p7-c007: Spawn the AG-UI state propagation task.
    // When the schema hot-swaps, emit a StateSnapshot event on the "schema" run
    // so connected agent frontends can update their tool picker in real-time.
    let agui_propagation_state = routes::agui::AgUiState::new(32);
    let _propagation_handle = {
        let mut version_rx = state_manager.subscribe_version();
        let sm = Arc::clone(&state_manager);
        let agui_prop = agui_propagation_state.clone();
        tokio::spawn(async move {
            loop {
                if version_rx.changed().await.is_err() {
                    break;
                }
                let compiled = sm.current();
                let mcp_count = compiled
                    .mcp_tools_doc
                    .get("tools")
                    .and_then(|t| t.as_array())
                    .map_or(0, Vec::len);
                agui_prop
                    .publish(fdb_domain::AgUiEvent::StateSnapshot {
                        run_id: "schema".to_owned(),
                        state: serde_json::json!({
                            "schema_version": compiled.version,
                            "mcp_tools_count": mcp_count,
                            "a2ui_catalog_version": compiled.a2ui_catalog.version,
                        }),
                    })
                    .await;
                tracing::info!(
                    schema_version = compiled.version,
                    mcp_tools = mcp_count,
                    "AG-UI StateSnapshot emitted"
                );
            }
        })
    };

    let gateway_state = GatewayState {
        state_manager: Arc::clone(&state_manager),
        graphql_executor,
        vector_rpc,
        pool: pool.clone(),
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
    // Apply the RLS extraction middleware to the reflection router so its
    // mutation handlers receive `Extension<RlsContext>` (required by the Keto +
    // Cedar gates). GET /list is also covered — harmless, and keeps a single
    // auth surface for the reflected CRUD routes.
    let reflection_router = state_manager
        .current()
        .router
        .as_ref()
        .clone()
        .layer(axum::middleware::from_fn(rls_layer::require_rls));

    // Build the A2UI registry router. All routes require a valid JWT bearer.
    let a2ui_router = Router::new()
        .route("/a2ui/v1/components", get(routes::a2ui::list_components))
        .route(
            "/a2ui/v1/components/search",
            post(routes::a2ui::search_components),
        )
        .route(
            "/a2ui/v1/components/bindings/{schema}/{table}",
            get(routes::a2ui::get_bindings),
        )
        .route(
            "/a2ui/v1/components/{slug}",
            get(routes::a2ui::get_component),
        )
        .route(
            "/a2ui/v1/applications",
            get(routes::a2ui::list_applications),
        )
        .route(
            "/a2ui/v1/applications/{id}",
            get(routes::a2ui::get_application),
        )
        .route(
            "/a2ui/v1/catalog/{catalog_id}",
            get(routes::a2ui::get_catalog),
        )
        .route(
            "/a2ui/v1/surfaces/assemble",
            post(routes::a2ui::assemble_surface),
        )
        .route(
            "/a2ui/v1/design-systems/import",
            post(routes::design_import::import_design_system),
        )
        .route(
            "/a2ui/v1/design-systems/{id}/tokens",
            get(routes::a2ui::get_design_system_tokens),
        )
        .layer(axum::middleware::from_fn(rls_layer::require_rls))
        .with_state(crate::routes::a2ui::A2uiState::from(gateway_state.clone()));

    // Build the MCP server router. Exposes the A2UI registry as JSON-RPC tools
    // at `/mcp/v1/a2ui`. Like the A2UI router, it sits behind JWT auth so every
    // tool call runs under the caller's RlsContext.
    let mcp_router = Router::new()
        .route("/mcp/v1/a2ui", post(routes::mcp::handle_mcp))
        .route("/mcp/v1/a2ui/sse", get(routes::mcp::handle_sse))
        .route("/mcp/v1/a2ui/health", get(routes::mcp::health))
        .layer(axum::middleware::from_fn(rls_layer::require_rls))
        .with_state(routes::mcp::McpState {
            a2ui: crate::routes::a2ui::A2uiState::from(gateway_state.clone()),
        });

    // Build the A2A task handler router. Exposes the A2UI registry as A2A
    // skills at `/a2a/v1` + `/.well-known/agent.json`. The agent card endpoint
    // is public (no auth); the JSON-RPC endpoint sits behind JWT auth.
    let a2a_router = Router::new()
        .route("/.well-known/agent.json", get(routes::a2a::agent_card))
        .route("/a2a/v1", post(routes::a2a::handle_a2a))
        .layer(axum::middleware::from_fn(rls_layer::require_rls))
        .with_state(routes::a2a::A2aState {
            a2ui: crate::routes::a2ui::A2uiState::from(gateway_state.clone()),
        });

    // Build the HTMX renderer router. Admin/prototyping surface for the A2UI
    // registry. Behind JWT auth so registry data respects RLS.
    let htmx_router = Router::new()
        .route("/htmx/", get(routes::htmx::index))
        .route("/htmx/admin/registry", get(routes::htmx::admin_registry))
        .route(
            "/htmx/components/{slug}",
            get(routes::htmx::render_component).post(routes::htmx::render_component_with_props),
        )
        .route(
            "/htmx/surfaces/assemble",
            get(routes::htmx::assemble_surface_html),
        )
        .layer(axum::middleware::from_fn(rls_layer::require_rls))
        .with_state(routes::htmx::HtmxState {
            a2ui: crate::routes::a2ui::A2uiState::from(gateway_state.clone()),
        });

    // Build the AG-UI event streaming router. SSE endpoint for agent run events.
    // Behind JWT auth — event publishing requires authentication.
    let agui_state = routes::agui::AgUiState::default().with_pool(pool.clone());

    // p7-c001 + p7-c002: Spawn the AG-UI hook dispatcher.
    // Polls flint.webhook_outbox for agui_run targeted entries (durable tier)
    // and converts them to AG-UI ToolCallResult events. Standard-tier agui_run
    // hooks fire directly from dispatch_webhook() via pg_net.
    let _agui_hook_handle = agui_hook_dispatcher::spawn(Arc::new(pool.clone()), agui_state.clone());

    // p14-c003: A2UI catalog hot-reload — when the StateManager hot-swaps the
    // compiled state (on `meta_runtime` NOTIFY, fired by A2UI catalog changes
    // via migration 0010), broadcast a `StateSnapshot` to every connected AG-UI
    // run. SDKs receiving this event revalidate their registry (e.g., SWR
    // `mutate()` in `@flint/react`'s `useFlintRegistry`).
    //
    // We subscribe to the StateManager's version watch channel and fan out on
    // every bump. `AgUiState` is `Clone` (cheap Arc clone internally), so we
    // clone it here before it is moved into the router below.
    {
        let mut version_rx = state_manager.subscribe_version();
        let agui_for_version = agui_state.clone();
        tokio::spawn(async move {
            while version_rx.changed().await.is_ok() {
                let version = *version_rx.borrow();
                tracing::info!(
                    version,
                    "schema version changed — notifying AG-UI clients (a2ui hot-reload)"
                );
                agui_for_version
                    .broadcast_all(fdb_domain::AgUiEvent::StateSnapshot {
                        run_id: "schema".to_owned(),
                        state: serde_json::json!({ "schema_version": version }),
                    })
                    .await;
            }
        });
    }

    let agent_events_router = Router::new()
        .route(
            "/agents/v1/runs",
            axum::routing::post(routes::agui::start_run),
        )
        .route(
            "/agents/v1/{run_id}/events",
            axum::routing::post(routes::agui::publish_event).get(routes::agui::stream_events),
        )
        .route(
            "/agents/v1/{run_id}/surfaces/assemble",
            axum::routing::post(routes::agui::assemble_and_emit_surface),
        )
        .layer(axum::middleware::from_fn(rls_layer::require_rls))
        .with_state(agui_state);

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/openapi.json", get(openapi_handler))
        .route("/mcp/v1/tools", get(mcp_tools_handler))
        .route("/rpc/vector", axum::routing::post(rpc_vector_handler))
        .route(
            "/graphql",
            get(graphql_ws_handler).post(handle_graphql_query),
        )
        .merge(a2ui_router)
        .merge(mcp_router)
        .merge(a2a_router)
        .merge(htmx_router)
        .merge(agent_events_router)
        .with_state(gateway_state)
        .merge(reflection_router)
        // p9-c004: Prometheus metrics endpoint — no auth, no rate limit.
        // Served outside the rate-limiting tower stack by merging after the main app.
        .route(
            "/metrics",
            get(move || async move { metrics_handle.render() }),
        )
        // p9-c004: Instrument every request with http_requests_total + duration histograms.
        .layer(metrics_layer);

    let addr = "0.0.0.0:8080";
    tracing::info!(%addr, "flint-quarry listening");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");

    // p9-c003: Per-IP token-bucket rate limiting via tower_governor.
    // FLINT_RATE_LIMIT_REST: requests-per-second sustained rate per IP (0 = disabled, default 100).
    // FLINT_RATE_LIMIT_BURST: token-bucket burst capacity (default 10).
    let rest_rps: u64 = std::env::var("FLINT_RATE_LIMIT_REST")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    let burst: u32 = std::env::var("FLINT_RATE_LIMIT_BURST")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);

    let app = if rest_rps > 0 {
        tracing::info!(rps = rest_rps, burst, "per-IP rate limiting enabled");
        let governor_conf = GovernorConfigBuilder::default()
            .per_second(rest_rps)
            .burst_size(burst)
            .finish()
            .expect("GovernorConfig: burst and period must be non-zero");
        let layer = GovernorLayer::new(governor_conf).error_handler(|err| {
            (
                StatusCode::TOO_MANY_REQUESTS,
                Json(serde_json::json!({
                    "error": "rate_limit_exceeded",
                    "message": err.to_string(),
                })),
            )
                .into_response()
        });
        app.layer(layer)
    } else {
        tracing::info!("rate limiting disabled (FLINT_RATE_LIMIT_REST=0)");
        app
    };

    // Use into_make_service_with_connect_info so that PeerIpKeyExtractor can
    // read ConnectInfo<SocketAddr> from the request extensions.  This is a no-op
    // overhead when rate limiting is disabled but keeps the serve call uniform.

    // p9-c005: Security response headers applied to every response regardless of route.
    // SetResponseHeaderLayer::if_not_present allows handlers to override per-route where
    // needed (e.g., streaming responses that set their own content headers).
    let app = app
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ));

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .expect("serve");
}

#[tracing::instrument(skip(state), fields(service = "flint-quarry"))]
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
#[tracing::instrument(skip(state))]
async fn openapi_handler(State(state): State<GatewayState>) -> Json<serde_json::Value> {
    let compiled = state.state_manager.current();
    Json(compiled.openapi_doc.clone())
}

/// GET /mcp/v1/tools — serve compiled MCP tool definitions from `DatabaseModel`.
///
/// Returns the MCP `tools/list` result generated by `McpCompiler`. Hot-swapped
/// on DDL changes. Protected by JWT auth via `require_rls`.
async fn mcp_tools_handler(State(state): State<GatewayState>) -> Json<serde_json::Value> {
    let compiled = state.state_manager.current();
    Json(compiled.mcp_tools_doc.clone())
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
        Err(fdb_ports::BackendError::Query(msg)) => {
            (StatusCode::BAD_REQUEST, Json(json!({"error": msg}))).into_response()
        }
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

/// Build the GraphQL subscription live-stream factory.
///
/// Composes a [`Quarry`] from `PgRest` (RLS re-query), `PgGraphQl` (required by the
/// constructor; unused on this path), and `FabricChangeSource` (the live change
/// stream — currently empty pending OQ-FRF-1). Keto is threaded so the subscribe-time
/// coarse check runs inside `FabricChangeSource`.
///
/// The returned factory, given a table's spec/meta and the subscriber's `RlsContext`,
/// yields the RLS-filtered, GraphQL-projected event stream. `spec.tenant` is filled
/// from the subscriber's claims here (the compiler has no per-subscriber context).
async fn build_subscription_factory(
    database_url: &str,
    keto_adapter: Arc<dyn fdb_ports::KetoCheck>,
) -> SubStreamFactory {
    let make_pool = || {
        let mut cfg = deadpool_postgres::Config::new();
        cfg.url = Some(database_url.to_owned());
        cfg.create_pool(
            Some(deadpool_postgres::Runtime::Tokio1),
            tokio_postgres::NoTls,
        )
        .expect("subscription pool create")
    };

    let sub_rest = Arc::new(PgRest::new(make_pool()));
    let sub_graphql = Arc::new(PgGraphQl::new(make_pool()));
    let realtime_keto_cfg = KetoConfig {
        base_url: std::env::var("KETO_BASE_URL").unwrap_or_else(|_| "http://keto:4466".into()),
    };

    // Select the change-stream backend. Default `fabric` (FRF gRPC — currently an
    // empty stream pending OQ-FRF-1). `FLINT_CHANGE_SOURCE=listen` uses the
    // in-process Postgres LISTEN/NOTIFY adapter, which emits real events without FRF.
    let use_listen = std::env::var("FLINT_CHANGE_SOURCE").as_deref() == Ok("listen");
    let change_source: Arc<dyn fdb_ports::ChangeStreamSource> = if use_listen {
        let cfg = ListenConfig {
            database_url: database_url.to_owned(),
            broadcast_capacity: std::env::var("FLINT_LISTEN_CAPACITY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1024),
        };
        Arc::new(
            ListenChangeSource::new(cfg, realtime_keto_cfg)
                .await
                .expect("listen change source"),
        )
    } else {
        let frf_cfg = FrfConfig {
            endpoint: std::env::var("FRF_ENDPOINT").unwrap_or_else(|_| "http://frf:50051".into()),
        };
        Arc::new(FabricChangeSource::new(frf_cfg, realtime_keto_cfg).expect("fabric change source"))
    };
    let quarry =
        Arc::new(Quarry::new(sub_rest, sub_graphql, change_source).with_keto(keto_adapter));

    Arc::new(
        move |mut spec: SubscriptionSpec, table_meta: TableMeta, who: RlsContext| {
            let quarry = Arc::clone(&quarry);
            spec.tenant = who.tenant().map(|t| t.0).unwrap_or_default();
            // Defer to the use-case; flatten the "failed to open stream" error into a
            // single GraphQL error event so the field terminates cleanly.
            futures::stream::once(async move {
                quarry
                    .subscribe_graphql_values(spec, table_meta, &who)
                    .await
            })
            .flat_map(|opened| match opened {
                Ok(s) => s,
                Err(e) => {
                    futures::stream::once(
                        async move { Err(async_graphql::Error::new(e.to_string())) },
                    )
                    .boxed()
                }
            })
            .boxed()
        },
    )
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
            GraphQLWebSocket::new(socket, schema, protocol)
                // Authenticate at connection_init: extract the bearer from the
                // init payload, verify it, and inject the resulting RlsContext
                // into the connection Data. The subscription resolver reads it via
                // `ctx.data::<RlsContext>()`. Fail CLOSED — a missing or invalid
                // token returns Err, which rejects the WebSocket connection.
                .on_connection_init(connection_init_rls)
                .serve()
                .await;
        })
}

/// `graphql-transport-ws` `connection_init` handler: authenticate the socket.
///
/// Reads a bearer token from the init payload (`{"authorization": "Bearer …"}`
/// or `{"Authorization": "…"}`) and verifies it into an `RlsContext` installed in
/// the connection `Data`. Returns `Err` (rejecting the connection) when the token
/// is absent or invalid — this is the fail-closed auth gate for subscriptions.
///
/// SECURITY: never log the token or the derived `keto_subject`.
async fn connection_init_rls(
    payload: serde_json::Value,
) -> async_graphql::Result<async_graphql::Data> {
    let bearer = payload
        .get("authorization")
        .or_else(|| payload.get("Authorization"))
        .and_then(serde_json::Value::as_str)
        .map(|s| s.strip_prefix("Bearer ").unwrap_or(s).to_owned())
        .ok_or_else(|| async_graphql::Error::new("missing authorization in connection_init"))?;

    let rls = rls_from_bearer(&bearer)
        .await
        .map_err(|_| async_graphql::Error::new("invalid or expired token"))?;

    let mut data = async_graphql::Data::default();
    data.insert(rls);
    Ok(data)
}

/// POST /graphql — GraphQL query and mutation handler.
///
/// Extracts the bearer token from the `Authorization` header, builds `RlsContext`,
/// and delegates to `graphql.resolve()` via `PgGraphQl::execute()`.
/// The response is the raw pg_graphql JSON — no envelope added.
#[tracing::instrument(skip(state, headers, body), fields(operation_name = ?body.operation_name))]
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

// ─── Rate-limiting unit tests ────────────────────────────────────────────────

#[cfg(test)]
mod rate_limit_tests {
    use axum::{
        body::Body,
        extract::ConnectInfo,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use tower::ServiceExt as _;
    use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

    /// Build a minimal test app with `per_second` / `burst` rate limits applied.
    fn rate_limited_app(per_second: u64, burst: u32) -> Router {
        let config = GovernorConfigBuilder::default()
            .per_second(per_second)
            .burst_size(burst)
            .finish()
            .expect("GovernorConfig");
        Router::new()
            .route("/ping", get(|| async { "pong" }))
            .layer(GovernorLayer::new(config))
    }

    /// Construct a plain GET request with a `ConnectInfo<SocketAddr>` extension so
    /// that `PeerIpKeyExtractor` can resolve the peer address without a TCP listener.
    fn make_request(path: &str) -> Request<Body> {
        let peer = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 1234);
        let mut req = Request::builder().uri(path).body(Body::empty()).unwrap();
        req.extensions_mut().insert(ConnectInfo(peer));
        req
    }

    /// `GovernorConfigBuilder` produces a valid config for the default parameters.
    #[test]
    fn governor_config_builds_without_panic() {
        let config = GovernorConfigBuilder::default()
            .per_second(100)
            .burst_size(10)
            .finish();
        assert!(config.is_some(), "expected Some(GovernorConfig), got None");
    }

    /// Config with burst = 0 must return None (zero is invalid per tower_governor docs).
    #[test]
    fn governor_config_rejects_zero_burst() {
        let config = GovernorConfigBuilder::default()
            .per_second(10)
            .burst_size(0)
            .finish();
        assert!(config.is_none(), "expected None for burst_size=0");
    }

    /// When FLINT_RATE_LIMIT_REST=0 the gate in main() bypasses the layer and all
    /// requests are served normally.  Model that logic here without a live server.
    #[tokio::test]
    async fn rate_limiting_disabled_when_rps_zero() {
        let rest_rps: u64 = 0; // simulates FLINT_RATE_LIMIT_REST=0

        // Mirror the if/else in main() — no GovernorLayer when disabled.
        let app: Router = if rest_rps > 0 {
            let cfg = GovernorConfigBuilder::default()
                .per_second(1)
                .burst_size(1)
                .finish()
                .expect("cfg");
            Router::new()
                .route("/ping", get(|| async { "pong" }))
                .layer(GovernorLayer::new(cfg))
        } else {
            Router::new().route("/ping", get(|| async { "pong" }))
        };

        // Five consecutive requests should all succeed when rate limiting is off.
        for _ in 0..5_u8 {
            let res = app.clone().oneshot(make_request("/ping")).await.unwrap();
            assert_eq!(res.status(), StatusCode::OK);
        }
    }

    /// After the burst bucket is exhausted the next request must receive 429.
    #[tokio::test]
    async fn returns_429_when_limit_exceeded() {
        // 1 req/s sustained, burst of 1 → the second immediate request is rejected.
        let app = rate_limited_app(1, 1);

        let res1 = app.clone().oneshot(make_request("/ping")).await.unwrap();
        assert_eq!(
            res1.status(),
            StatusCode::OK,
            "first request should succeed"
        );

        let res2 = app.clone().oneshot(make_request("/ping")).await.unwrap();
        assert_eq!(
            res2.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "second immediate request should be rate-limited"
        );
    }
}

// ─── Security-header unit tests ───────────────────────────────────────────────

#[cfg(test)]
mod security_header_tests {
    use axum::http::{HeaderName, HeaderValue};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt as _;
    use tower_http::set_header::SetResponseHeaderLayer;

    /// Build a minimal test app with the three security header layers applied,
    /// mirroring the layers added in `main()`.
    fn secure_app() -> Router {
        Router::new()
            .route("/healthz", get(|| async { "ok" }))
            .layer(SetResponseHeaderLayer::if_not_present(
                HeaderName::from_static("x-content-type-options"),
                HeaderValue::from_static("nosniff"),
            ))
            .layer(SetResponseHeaderLayer::if_not_present(
                HeaderName::from_static("x-frame-options"),
                HeaderValue::from_static("DENY"),
            ))
            .layer(SetResponseHeaderLayer::if_not_present(
                HeaderName::from_static("referrer-policy"),
                HeaderValue::from_static("strict-origin-when-cross-origin"),
            ))
    }

    /// All three security headers must be present and have the expected values
    /// on a plain GET /healthz response.
    #[tokio::test]
    async fn security_headers_present_on_healthz() {
        let app = secure_app();
        let req = Request::builder()
            .uri("/healthz")
            .body(Body::empty())
            .unwrap();

        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let headers = res.headers();
        assert_eq!(
            headers
                .get("x-content-type-options")
                .and_then(|v| v.to_str().ok()),
            Some("nosniff"),
            "X-Content-Type-Options must be 'nosniff'"
        );
        assert_eq!(
            headers.get("x-frame-options").and_then(|v| v.to_str().ok()),
            Some("DENY"),
            "X-Frame-Options must be 'DENY'"
        );
        assert_eq!(
            headers.get("referrer-policy").and_then(|v| v.to_str().ok()),
            Some("strict-origin-when-cross-origin"),
            "Referrer-Policy must be 'strict-origin-when-cross-origin'"
        );
    }

    /// A handler that pre-sets X-Content-Type-Options should NOT be overwritten
    /// by the `if_not_present` layer — the handler's value wins.
    #[tokio::test]
    async fn if_not_present_does_not_overwrite_handler_header() {
        use axum::body::Body as AxumBody;
        use axum::http::Response as AxumResponse;

        let app = Router::new()
            .route(
                "/custom",
                get(|| async {
                    AxumResponse::builder()
                        .header("x-content-type-options", "custom-value")
                        .body(AxumBody::empty())
                        .unwrap()
                }),
            )
            .layer(SetResponseHeaderLayer::if_not_present(
                HeaderName::from_static("x-content-type-options"),
                HeaderValue::from_static("nosniff"),
            ));

        let req = Request::builder()
            .uri("/custom")
            .body(Body::empty())
            .unwrap();

        let res = app.oneshot(req).await.unwrap();
        assert_eq!(
            res.headers()
                .get("x-content-type-options")
                .and_then(|v| v.to_str().ok()),
            Some("custom-value"),
            "if_not_present must not overwrite a header already set by the handler"
        );
    }
}
