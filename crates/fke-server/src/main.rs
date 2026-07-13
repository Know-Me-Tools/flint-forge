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
#[cfg(test)]
mod tests;

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
use forge_identity::RlsContext;
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
    /// `did:prometheus:`-scheme verifier (in-memory/HTTP key resolution, no
    /// per-call network cost after the first). Constructed once and shared —
    /// `VerifierDid` holds a TTL key cache that must persist across calls.
    verifier_did: Arc<fke_sign_did::VerifierDid>,
    /// Cosign/Sigstore verifier (Rekor transparency-log lookup keyed by
    /// content digest). Constructed once for a shared `reqwest::Client`.
    verifier_cosign: Arc<fke_sign_cosign::VerifierCosign>,
}

/// Dispatch to the verifier matching `manifest.publisher_did`'s scheme, and
/// reject outright when a `did:prometheus:` manifest carries no signature
/// (`VerifierCosign` doesn't need one — it looks the signature up from Rekor
/// by content digest — so only the DID path can fail this check before
/// dispatch).
///
/// p16-c002: this is the supply-chain trust gate. It must run (a) at
/// register, so an unsigned/invalid upload is rejected before it is ever
/// stored, and (b) at invoke on every cold cache-load (see `invoke_impl`) —
/// the same lifecycle as the WASM bytes cache itself, so a component is
/// re-verified independently of whatever checks ran at register, without
/// paying a full verification cost (a Rekor HTTP round-trip, for Cosign) on
/// every single request.
async fn verify_manifest_signature(
    state: &KilnState,
    manifest: &fke_domain::FunctionManifest,
    artifact: &[u8],
) -> Result<(), fke_ports::SignError> {
    use base64::Engine as _;
    use fke_ports::SignatureVerifier;

    if manifest.publisher_did.starts_with("did:prometheus:") {
        let sig_b64 = manifest
            .signature_b64
            .as_deref()
            .ok_or(fke_ports::SignError::Unsigned)?;
        let sig_bytes = base64::engine::general_purpose::STANDARD
            .decode(sig_b64)
            .map_err(|_| fke_ports::SignError::Invalid)?;
        state
            .verifier_did
            .verify(manifest, &sig_bytes, artifact)
            .await
    } else {
        // Cosign path: the verifier fetches its own signature material from
        // Rekor by content digest, so the (possibly absent) signature_b64 is
        // irrelevant here — pass an empty slice.
        state.verifier_cosign.verify(manifest, &[], artifact).await
    }
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

    let db_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/flint".into());

    let pool = sqlx::PgPool::connect(&db_url)
        .await
        .expect("database connect");

    // Cedar policy engine backed by flint_kiln.cedar_policies (p7b-c002).
    // Falls back to SourceUnavailable (deny-all) if the DB is unreachable.
    // kiln_policy::TestAllowAllPolicySource is #[cfg(test)]-gated and unavailable here.
    let policy_source = Arc::new(kiln_db_policy::DbKilnPolicySource::new(pool.clone()));
    let pep: Arc<dyn forge_policy::Pep> =
        Arc::new(forge_policy::CedarPolicyEngine::new(policy_source).await);
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
    let state = KilnState {
        runtime,
        store,
        registry,
        verifier_did: Arc::new(fke_sign_did::VerifierDid::new()),
        verifier_cosign: Arc::new(fke_sign_cosign::VerifierCosign::new()),
    };

    let plane = if cfg!(feature = "control-plane") {
        "control"
    } else {
        "data"
    };

    // p14-c005: Prometheus metrics layer + /metrics handler.
    let (metric_layer, metric_handle) = PrometheusMetricLayer::pair();

    let mut app = Router::new()
        .route("/healthz", get(healthz))
        .route(
            "/metrics",
            get(move || std::future::ready(metric_handle.render())),
        )
        .route("/functions/v1/{name}", any(invoke_function))
        .route(
            "/functions/v1/{name}@{version}",
            any(invoke_function_versioned),
        )
        .layer(metric_layer);

    if cfg!(feature = "control-plane") {
        app = app.route(
            "/admin/functions",
            post(register_function).get(list_functions),
        );
    }

    let app = app.with_state(state);

    let addr = "0.0.0.0:8090";
    tracing::info!(%addr, plane, "flint-kiln listening");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    axum::serve(listener, app.into_make_service())
        .await
        .expect("serve");
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

    // p16-c003: a valid bearer is now mandatory on this data-plane HTTP path
    // — `caller = None` must never reach `EdgeRuntime::handle_with_telemetry`
    // from here, since that's exactly the case that used to silently skip
    // the Cedar `kiln:invoke`/capability gates. (The Kiln BGW's system-level
    // invocations are a separate call path — `kiln_bgw.rs` calls
    // `EdgeRuntime::handle` directly with a synthesized caller identity, not
    // through this HTTP handler — so this doesn't affect it.)
    let Some(bearer) = extract_bearer(&headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "missing Authorization header"})),
        )
            .into_response();
    };
    let Ok(caller_rls) = fdb_auth::rls_from_bearer(&bearer).await else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "invalid or expired token"})),
        )
            .into_response();
    };
    let caller = Some(&caller_rls);

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
    if let Err(resp) = ensure_loaded_and_verified(state, &manifest, &content_id, name, version).await {
        return resp;
    }

    // 3. Invoke with granted capabilities (all declared caps; Cedar gate is future p6-c005)
    let request = KilnRequest {
        method: "POST".into(),
        uri: format!("/functions/v1/{name}"),
        headers: headers
            .iter()
            .filter_map(|(k, v)| {
                v.to_str()
                    .ok()
                    .map(|s| (k.as_str().to_owned(), s.to_owned()))
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

/// Load `content_id`'s WASM bytes into the runtime cache on a cold miss,
/// verifying the manifest's signature first (p16-c002) — extracted from
/// `invoke_impl` to keep it under the line-count lint; also makes the
/// "verify once per cache-load, not once per request" lifecycle explicit as
/// its own unit.
async fn ensure_loaded_and_verified(
    state: &KilnState,
    manifest: &fke_domain::FunctionManifest,
    content_id: &ContentId,
    name: &str,
    version: &str,
) -> Result<(), axum::response::Response> {
    use fke_ports::ComponentStore;

    if state.runtime.is_loaded(content_id) {
        return Ok(());
    }

    let wasm_bytes = match state.store.get(content_id).await {
        Ok(bytes) => bytes,
        Err(fke_ports::StoreError::NotFound) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(json!({"error": "artifact not found"})),
            )
                .into_response());
        }
        Err(e) => {
            tracing::error!(error = %e, "store get failed");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "store error"})),
            )
                .into_response());
        }
    };

    if let Err(e) = verify_manifest_signature(state, manifest, &wasm_bytes).await {
        tracing::warn!(name, version, error = %e, "signature verification failed at load");
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "signature verification failed"})),
        )
            .into_response());
    }

    if let Err(e) = state.runtime.load_wasm(content_id.clone(), &wasm_bytes) {
        tracing::error!(error = %e, "load_wasm failed");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "runtime load failed"})),
        )
            .into_response());
    }

    Ok(())
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

/// The Postgres/JWT role this repo treats as privileged everywhere else
/// (`ext-flint-meta`, `fdb-gateway`'s policy source — see their doc comments)
/// — reused here so `/admin/functions` matches the same admin convention
/// rather than inventing a Kiln-specific one.
const ADMIN_ROLE: &str = "service_role";

/// p16-c003: require a valid bearer AND an admin-scoped role for
/// `/admin/functions` — previously this route had no auth middleware at all,
/// gated only by the compile-time `control-plane` feature flag (which
/// controls whether the route is mounted, not who may call it).
async fn require_admin(headers: &HeaderMap) -> Result<RlsContext, axum::response::Response> {
    let Some(bearer) = extract_bearer(headers) else {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "missing Authorization header"})),
        )
            .into_response());
    };
    let Ok(caller) = fdb_auth::rls_from_bearer(&bearer).await else {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "invalid or expired token"})),
        )
            .into_response());
    };
    if caller.role != ADMIN_ROLE {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "admin role required"})),
        )
            .into_response());
    }
    Ok(caller)
}

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
    headers: HeaderMap,
    Json(body): Json<RegisterBody>,
) -> impl IntoResponse {
    use base64::Engine as _;
    use fke_ports::ComponentStore;

    if let Err(resp) = require_admin(&headers).await {
        return resp;
    }

    let Ok(wasm_bytes) = base64::engine::general_purpose::STANDARD.decode(&body.wasm_base64) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "invalid base64 wasm_base64"})),
        )
            .into_response();
    };

    // p16-c002: reject unsigned/invalid components before they are ever
    // stored or made resolvable.
    if let Err(e) = verify_manifest_signature(&state, &body.manifest, &wasm_bytes).await {
        tracing::warn!(name = %body.name, version = %body.version, error = %e, "signature verification failed at register");
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "signature verification failed"})),
        )
            .into_response();
    }

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
    .bind(
        content_id
            .0
            .strip_prefix("sha256:")
            .unwrap_or(&content_id.0),
    )
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

async fn list_functions(State(state): State<KilnState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(resp) = require_admin(&headers).await {
        return resp;
    }

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
