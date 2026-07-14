//! Composition-root wiring: pools, adapters, gates, routes, server startup.
//!
//! Relocated from `main.rs`'s `fn main()` body (p16 file-size split) — behavior
//! unchanged. `main()` in `main.rs` remains the actual `#[tokio::main]` binary
//! entry point and simply delegates to [`run`].

use std::sync::Arc;

use axum::{
    http::{HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use fdb_gateway::a2ui_embedder;
use fdb_postgres::{PgGraphQl, PgRest, PgVectorRpc};
use fdb_reflection::MutationGates;
use fdb_reflection::{ReflectionEngine, StateManager};
use forge_policy::CedarPolicyEngine;
use sqlx::PgPool;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::graphql::handle_graphql_query;
use crate::handlers::{healthz, mcp_tools_handler, openapi_handler, rpc_vector_handler};
use crate::subscriptions::{build_subscription_factory, graphql_ws_handler};
use crate::GatewayState;
use crate::{agui_hook_dispatcher, keto_sync, policy_source, rls_layer, routes, telemetry};

// Composition root: sequential wiring of pools, adapters, gates, routes. Called
// from the anyhow-at-the-edge binary entry point (`main()` in `main.rs`); a
// long linear body is idiomatic here.
#[allow(clippy::too_many_lines)]
pub(crate) async fn run() {
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

    // Build the REST/`rpc` executor for the reflection compiler. Same
    // `PgBackend::acquire(rls)`-gated pattern as `graphql_executor`/`vector_rpc`
    // above — every statement `fdb-reflection`'s CRUD/`rpc` handlers run goes
    // through this pool's `acquire()`, which issues the RLS `SET LOCAL` GUCs
    // before the query. This pool MUST NOT be the raw `pool` (sqlx, above) —
    // that one never sets RLS context and is reserved for migrations/A2UI
    // seed/embedder writes. (p16-c001: REST/RPC RLS enforcement.)
    let rest_pool = {
        let mut cfg = deadpool_postgres::Config::new();
        cfg.url = Some(database_url.clone());
        cfg.create_pool(
            Some(deadpool_postgres::Runtime::Tokio1),
            tokio_postgres::NoTls,
        )
        .expect("rest-executor pool create")
    };
    let rest_executor: Arc<dyn fdb_ports::SqlExecutor> = Arc::new(PgRest::new(rest_pool));

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
            rest_executor,
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
            "/a2ui/v1/catalog/{*catalog_id}",
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
