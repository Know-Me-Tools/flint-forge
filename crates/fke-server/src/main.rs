//! Flint Kiln server — data-plane (default) + control-plane (`control-plane` feature).
//!
//! Data-plane:
//!   `POST /functions/v1/{name}` — invoke a registered function
//!   `GET  /healthz`             — health check
//!
//! Control-plane (feature=control-plane):
//!   `POST /admin/functions`     — register a function (upload + compile)
//!   `GET  /admin/functions`     — list registered functions
#![forbid(unsafe_code)]

mod kiln_bgw;
mod kiln_db_policy;
mod kiln_policy;

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    routing::{any, get, post},
    Router,
};
use axum_prometheus::PrometheusMetricLayer;
use fke_domain::ContentId;
use fke_registry::PgComponentStore;
use fke_runtime::{EdgeRuntime, KilnRequest};
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig as _;
use opentelemetry_sdk::trace::SdkTracerProvider;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter};

/// Shared server state.
#[derive(Clone)]
struct KilnState {
    runtime: Arc<EdgeRuntime>,
    store: Arc<PgComponentStore>,
    registry: Arc<fke_registry::PgRegistry>,
}

#[tokio::main]
async fn main() {
    // p9-c004: init tracing (fmt + optional OTLP). Guard held for process lifetime.
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    let registry = tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(true));
    let _tracer_provider: Option<SdkTracerProvider> =
        if let Ok(endpoint) = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
            let exporter = opentelemetry_otlp::SpanExporter::builder()
                .with_http()
                .with_endpoint(endpoint.clone())
                .build()
                .expect("OTLP span exporter");
            let provider = SdkTracerProvider::builder()
                .with_batch_exporter(exporter)
                .build();
            let tracer = provider.tracer("fke-server");
            registry.with(OpenTelemetryLayer::new(tracer)).init();
            tracing::info!(endpoint, "OTLP tracing enabled");
            Some(provider)
        } else {
            registry.init();
            None
        };

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/flint".into());

    let pool = sqlx::PgPool::connect(&db_url)
        .await
        .expect("database connect");

    // Cedar policy engine backed by flint_kiln.cedar_policies (p7b-c002).
    // Falls back to SourceUnavailable (deny-all) if the DB is unreachable.
    // kiln_policy::TestAllowAllPolicySource is #[cfg(test)]-gated and unavailable here.
    let policy_source = Arc::new(kiln_db_policy::DbKilnPolicySource::new(pool.clone()));
    let pep: Arc<dyn forge_policy::Pep> = Arc::new(
        forge_policy::CedarPolicyEngine::new(policy_source).await,
    );
    let runtime = Arc::new(EdgeRuntime::new().expect("EdgeRuntime::new").with_pep(pep));
    let store = Arc::new(PgComponentStore::new(pool.clone()));
    let registry = Arc::new(fke_registry::PgRegistry::new(pool.clone()));

    // Spawn the Kiln BGW — drains flint.webhook_outbox WHERE target_type='kiln'.
    let _kiln_bgw = kiln_bgw::spawn(
        Arc::new(pool.clone()),
        Arc::clone(&runtime),
        Arc::clone(&registry),
        Arc::clone(&store),
    );
    let state = KilnState { runtime, store, registry };

    let plane = if cfg!(feature = "control-plane") {
        "control"
    } else {
        "data"
    };

    // p14-c005: Prometheus metrics layer + /metrics handler.
    let (metric_layer, metric_handle) = PrometheusMetricLayer::pair();

    let mut app = Router::new()
        .route("/healthz", get(healthz))
        .route("/metrics", get(move || std::future::ready(metric_handle.render())))
        .route("/functions/v1/{name}", any(invoke_function))
        .route("/functions/v1/{name}@{version}", any(invoke_function_versioned))
        .layer(metric_layer);

    if cfg!(feature = "control-plane") {
        app = app
            .route("/admin/functions", post(register_function).get(list_functions));
    }

    let app = app.with_state(state);

    let addr = "0.0.0.0:8090";
    tracing::info!(%addr, plane, "flint-kiln listening");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    axum::serve(listener, app.into_make_service()).await.expect("serve");
}

// ─── Healthz ─────────────────────────────────────────────────────────────────

async fn healthz() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "flint-kiln",
        "plane": if cfg!(feature = "control-plane") { "control" } else { "data" }
    }))
}

// ─── Data-plane: invoke ───────────────────────────────────────────────────────

#[tracing::instrument(skip(state, headers, body), fields(function = %name))]
async fn invoke_function(
    State(state): State<KilnState>,
    Path(name): Path<String>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    invoke_impl(&state, &name, "latest", headers, body).await
}

#[tracing::instrument(skip(state, headers, body), fields(function = %name, version = %version))]
async fn invoke_function_versioned(
    State(state): State<KilnState>,
    Path((name, version)): Path<(String, String)>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    invoke_impl(&state, &name, &version, headers, body).await
}

async fn invoke_impl(
    state: &KilnState,
    name: &str,
    version: &str,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> axum::response::Response {
    use fke_ports::ComponentRegistry;

    // Extract optional caller identity from Authorization header.
    // Present on direct HTTP invocations; absent on BGW (system-level) calls.
    // The Cedar gate in EdgeRuntime is skipped when caller = None.
    let caller_rls = if let Some(bearer) = extract_bearer(&headers) {
        fdb_auth::rls_from_bearer(&bearer).await.ok()
    } else {
        None
    };
    let caller = caller_rls.as_ref();

    // 1. Resolve manifest
    let manifest = match state.registry.resolve(name, version).await {
        Ok(m) => m,
        Err(fke_ports::StoreError::NotFound) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": format!("function {name}@{version} not found")})),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!(name, version, error = %e, "registry resolve failed");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "registry error"})),
            )
                .into_response();
        }
    };

    // 2. Load WASM bytes from store (load once into runtime cache per content_digest)
    let content_id = ContentId(format!("sha256:{}", manifest.content_digest));
    {
        use fke_ports::ComponentStore;
        if !state.runtime.is_loaded(&content_id) {
            match state.store.get(&content_id).await {
                Ok(wasm_bytes) => {
                    if let Err(e) = state.runtime.load_wasm(content_id.clone(), &wasm_bytes) {
                        tracing::error!(error = %e, "load_wasm failed");
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({"error": "runtime load failed"})),
                        )
                            .into_response();
                    }
                }
                Err(fke_ports::StoreError::NotFound) => {
                    return (
                        StatusCode::NOT_FOUND,
                        Json(json!({"error": "artifact not found"})),
                    )
                        .into_response();
                }
                Err(e) => {
                    tracing::error!(error = %e, "store get failed");
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "store error"})),
                    )
                        .into_response();
                }
            }
        }
    }

    // 3. Invoke with granted capabilities (all declared caps; Cedar gate is future p6-c005)
    let request = KilnRequest {
        method: "POST".into(),
        uri: format!("/functions/v1/{name}"),
        headers: headers
            .iter()
            .filter_map(|(k, v)| {
                v.to_str().ok().map(|s| (k.as_str().to_owned(), s.to_owned()))
            })
            .collect(),
        body: body.to_vec(),
    };

    // p14-c005: per-function invocation counter.
    metrics::counter!("kiln_invocations_total", "function" => name.to_owned()).increment(1);

    match state
        .runtime
        .handle_with_telemetry(&content_id, &manifest.capabilities, caller, request)
        .await
    {
        Ok(outcome) => (
            StatusCode::from_u16(outcome.response.status).unwrap_or(StatusCode::OK),
            [(header::CONTENT_TYPE, "application/json")],
            outcome.response.body,
        )
            .into_response(),
        Err(e) => {
            tracing::error!(name, error = %e, "function invocation error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "invocation error", "details": e.to_string()})),
            )
                .into_response()
        }
    }
}

/// Extract the raw bearer token from an `Authorization: Bearer <token>` header.
fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(str::to_owned)
}

// ─── Control-plane: register + list ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RegisterBody {
    name: String,
    version: String,
    manifest: fke_domain::FunctionManifest,
    /// Raw WASM component bytes (base64-encoded).
    wasm_base64: String,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
struct RegisterResponse {
    function_id: String,
    content_digest: String,
}

#[allow(unused_variables)]
async fn register_function(
    State(state): State<KilnState>,
    Json(body): Json<RegisterBody>,
) -> impl IntoResponse {
    use base64::Engine as _;
    use fke_ports::ComponentStore;

    let Ok(wasm_bytes) = base64::engine::general_purpose::STANDARD.decode(&body.wasm_base64) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "invalid base64 wasm_base64"})),
        )
            .into_response();
    };

    let content_id = match state.store.put(&wasm_bytes).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!(error = %e, "store put failed");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "store error"})),
            )
                .into_response();
        }
    };

    // Insert into registry
    let result: Result<(String,), _> = sqlx::query_as(
        "INSERT INTO flint_kiln.functions (name, version, content_digest, manifest)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (name, version) DO UPDATE
         SET content_digest = EXCLUDED.content_digest,
             manifest       = EXCLUDED.manifest
         RETURNING id::text",
    )
    .bind(&body.name)
    .bind(&body.version)
    .bind(content_id.0.strip_prefix("sha256:").unwrap_or(&content_id.0))
    .bind(serde_json::to_value(&body.manifest).unwrap_or(Value::Null))
    .fetch_one(state.store.pool())
    .await;

    match result {
        Ok((id,)) => Json(json!({
            "function_id": id,
            "content_digest": content_id.0,
        }))
        .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "register function failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "registry error"})),
            )
                .into_response()
        }
    }
}

async fn list_functions(State(state): State<KilnState>) -> impl IntoResponse {
    let rows: Result<Vec<(String, String, String, bool)>, _> = sqlx::query_as(
        "SELECT id::text, name, version, active
         FROM flint_kiln.functions
         ORDER BY name, version",
    )
    .fetch_all(state.store.pool())
    .await;

    match rows {
        Ok(rows) => {
            let functions: Vec<Value> = rows
                .into_iter()
                .map(|(id, name, version, active)| {
                    json!({"id": id, "name": name, "version": version, "active": active})
                })
                .collect();
            Json(json!({"functions": functions})).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "list functions failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "list failed"})),
            )
                .into_response()
        }
    }
}
