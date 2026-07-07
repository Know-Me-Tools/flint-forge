//! Telemetry initialisation for fdb-gateway.
//!
//! Initialises two pillars:
//!
//! **Tracing (OTLP)**
//! When `OTEL_EXPORTER_OTLP_ENDPOINT` is set the subscriber gains a
//! `tracing_opentelemetry` layer that exports spans via OTLP/HTTP-JSON
//! to an OpenTelemetry Collector.  When the variable is absent a
//! no-op tracer is installed — zero overhead, no network dependency.
//!
//! **Metrics (Prometheus)**
//! `axum_prometheus::PrometheusMetricLayer` is installed as an Axum tower
//! middleware in `main.rs`.  The matching handle is returned from
//! `metrics_handle()` and served on `GET /metrics`.
//!
//! # Shutdown
//! Callers hold the returned `TelemetryGuard` for the lifetime of the binary.
//! Dropping the guard flushes and shuts down the OTLP exporter.
#![forbid(unsafe_code)]

use axum_prometheus::{metrics_exporter_prometheus::PrometheusHandle, PrometheusMetricLayer};
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig as _;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter};

/// Held by the binary's `main`.  Dropping triggers OTLP shutdown flush.
pub struct TelemetryGuard {
    tracer_provider: Option<SdkTracerProvider>,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Some(provider) = self.tracer_provider.take() {
            if let Err(e) = provider.shutdown() {
                // Best-effort: log but do not panic on shutdown flush errors.
                eprintln!("OTLP tracer provider shutdown error: {e}");
            }
        }
    }
}

/// Initialise tracing subscriber (fmt + optional OTLP) and return a guard.
///
/// Call **once** at the top of `main()` before any tracing macros.
pub fn init_tracing() -> TelemetryGuard {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());

    let registry = tracing_subscriber::registry().with(filter).with(
        tracing_subscriber::fmt::layer().with_target(true).with_thread_ids(false),
    );

    if let Ok(endpoint) = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
        // OTLP/HTTP-JSON exporter — uses reqwest (already in workspace), avoids tonic
        // version conflicts that would arise from the gRPC transport.
        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_endpoint(endpoint.clone())
            .build()
            .expect("OTLP span exporter build failed");

        let provider = SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .build();

        let tracer = provider.tracer("fdb-gateway");
        let otel_layer = OpenTelemetryLayer::new(tracer);

        registry.with(otel_layer).init();
        tracing::info!(endpoint, "OTLP tracing enabled");

        TelemetryGuard {
            tracer_provider: Some(provider),
        }
    } else {
        registry.init();
        tracing::debug!("OTEL_EXPORTER_OTLP_ENDPOINT not set; OTLP tracing disabled");
        TelemetryGuard {
            tracer_provider: None,
        }
    }
}

/// Build the Prometheus metric layer + handle.
///
/// Install the returned `layer` into the Axum `Router` before `with_state`.
/// Serve `handle.render()` on `GET /metrics`.
#[must_use]
pub fn metrics_layer() -> (PrometheusMetricLayer<'static>, PrometheusHandle) {
    PrometheusMetricLayer::pair()
}

/// Spawn a background loop that emits sqlx pool gauges every 15 seconds.
///
/// Polls `pool.size()` and `pool.num_idle()` and exports them as Prometheus
/// gauges so the Grafana "Active DB Connections" panel and the
/// `HighDbConnections` alert rule produce real data.
///
/// Call once after pool creation in `main()`:
///   `telemetry::spawn_pool_metrics(pool.clone());`
pub fn spawn_pool_metrics(pool: sqlx::PgPool) {
    use std::time::Duration;

    // Register metric descriptions (one-time).
    metrics::describe_gauge!(
        "sqlx_pool_connections_open",
        "Total connections currently held by the pool"
    );
    metrics::describe_gauge!(
        "sqlx_pool_connections_idle",
        "Idle connections available for immediate use"
    );

    tokio::spawn(async move {
        // Offset by 2 s to avoid colliding with the Prometheus scrape tick.
        let mut interval = tokio::time::interval(Duration::from_secs(15));
        interval.tick().await; // consume the immediate first tick

        loop {
            interval.tick().await;
            let size = pool.size();
            let idle = pool.num_idle();
            metrics::gauge!("sqlx_pool_connections_open").set(f64::from(size));
            let idle: u32 = idle.try_into().unwrap_or(0);
            metrics::gauge!("sqlx_pool_connections_idle").set(f64::from(idle));
        }
    });
}
