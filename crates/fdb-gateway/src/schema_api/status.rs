//! `GET /schema/v1/status` (FFS-001 §4.5): enabled flag, allowlist,
//! reflection schema version, and the most recent apply outcome — served
//! without touching any user schema, so it works before any provisioning
//! has ever happened.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use super::{
    epoch_to_iso8601, error_response, require_enabled, require_provisioner, SchemaApiState,
};

/// Handler for `GET /schema/v1/status`.
pub async fn status(
    State(state): State<SchemaApiState>,
    headers: HeaderMap,
) -> Response {
    let provisioner = match require_enabled(&state) {
        Ok(p) => p,
        Err(resp) => return *resp,
    };
    if let Err(resp) = require_provisioner(&headers).await {
        return resp;
    }

    let last_apply = match provisioner.last_apply().await {
        Ok(summary) => summary.map(|s| {
            json!({
                "planId": s.plan_id,
                "at": epoch_to_iso8601(s.at_epoch),
                "status": s.status,
            })
        }),
        Err(e) => {
            tracing::error!(error = %e, "ledger summary query failed");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "ledger query failed");
        }
    };

    (
        StatusCode::OK,
        Json(json!({
            "enabled": true,
            "namespaces": *state.namespaces,
            "schemaVersion": state.state_manager.current().version,
            "lastApply": last_apply,
        })),
    )
        .into_response()
}
