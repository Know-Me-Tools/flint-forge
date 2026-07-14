//! `GET /healthz` — health check.

use axum::response::Json;
use serde_json::{json, Value};

pub(crate) async fn healthz() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "flint-kiln",
        "plane": if cfg!(feature = "control-plane") { "control" } else { "data" }
    }))
}
