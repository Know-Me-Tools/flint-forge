//! Flint Kiln server. Admin REST (control plane, behind `control-plane` feature) + /functions/v1 (data plane).
#![forbid(unsafe_code)]

use axum::{
    response::Json,
    routing::{any, get, post},
    Router,
};
use serde_json::json;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let plane = if cfg!(feature = "control-plane") {
        "control"
    } else {
        "data"
    };

    let mut app = Router::new()
        .route(
            "/healthz",
            get(move || async move {
                Json(json!({"status":"ok","service":"flint-kiln","plane":plane}))
            }),
        )
        .route(
            "/functions/v1/{name}",
            any(|| async { "edge invoke (stub)" }),
        );

    if cfg!(feature = "control-plane") {
        app = app.route("/admin/functions", post(|| async { "register (stub)" }));
    }

    let addr = "0.0.0.0:8090";
    tracing::info!(%addr, plane, "flint-kiln listening");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}
