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

mod handlers;
mod kiln_bgw;
mod kiln_db_policy;
mod kiln_policy;
mod state;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use axum::{
    routing::{any, get, post},
    Router,
};
use axum_prometheus::PrometheusMetricLayer;
use fke_registry::PgComponentStore;
use fke_runtime::EdgeRuntime;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig as _;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter};

use handlers::admin::{list_functions, register_function};
use handlers::health::healthz;
use handlers::invoke::{invoke_function, invoke_function_versioned};
use state::KilnState;

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
