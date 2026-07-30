//! `POST /schema/v1/plan` — pure planner (FFS-001 §4.2).
//!
//! Deserialize the typed spec (closed grammar — injection dies here) →
//! reserved/allowlist gate → introspect the namespace → `generate()` →
//! persist ONE `flint_schema` ledger row (`status='planned'`, upserted by
//! hash — the durable plan store, plan.md D-P1) → return the reviewable
//! plan. No user-schema object is ever touched.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use fdb_app::provision::generate;
use fdb_domain::provision::{PlanId, PlannedRecord, SchemaSpec};
use serde_json::json;
use uuid::Uuid;

use super::{
    epoch_to_iso8601, error_response, now_epoch, require_allowlisted, require_enabled,
    require_provisioner, SchemaApiState, PLAN_TTL_SECS,
};

/// Handler for `POST /schema/v1/plan`.
pub async fn plan(
    State(state): State<SchemaApiState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let provisioner = match require_enabled(&state) {
        Ok(p) => p,
        Err(resp) => return *resp,
    };
    let _caller = match require_provisioner(&headers).await {
        Ok(ctx) => ctx,
        Err(resp) => return resp,
    };

    let spec: SchemaSpec = match serde_json::from_slice(&body) {
        Ok(spec) => spec,
        Err(e) => {
            return error_response(StatusCode::BAD_REQUEST, &format!("invalid spec: {e}"));
        }
    };

    if let Err(resp) = require_allowlisted(&state, spec.namespace.as_str()) {
        return *resp;
    }

    let live = match provisioner.introspect_namespace(&spec.namespace).await {
        Ok(live) => live,
        Err(e) => {
            tracing::error!(namespace = %spec.namespace, error = %e, "introspection failed");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "introspection failed");
        }
    };

    let generated = match generate(&spec, &live) {
        Ok(plan) => plan,
        Err(e) => {
            // PlanError Display names identifiers, never secrets.
            return error_response(StatusCode::BAD_REQUEST, &e.to_string());
        }
    };

    // Reuse an unexpired stored plan with the same hash rather than minting
    // duplicates; otherwise mint and persist a fresh planned row.
    let now = now_epoch();
    let (plan_id, created_epoch) = match provisioner.load_planned(&generated.hash).await {
        Ok(Some(stored))
            if stored.status == "planned" && now - stored.created_at_epoch < PLAN_TTL_SECS =>
        {
            (stored.plan_id, stored.created_at_epoch)
        }
        Ok(_) => {
            let minted = PlanId(format!("pln_{}", Uuid::new_v4().simple()));
            let record = PlannedRecord {
                plan_id: minted.clone(),
                hash: generated.hash.clone(),
                namespace: generated.namespace.clone(),
                spec: spec.clone(),
                ddl: generated.ddl.clone(),
            };
            if let Err(e) = provisioner.persist_planned(&record).await {
                tracing::error!(plan_id = %minted, error = %e, "plan persistence failed");
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "plan persistence failed",
                );
            }
            (minted, now)
        }
        Err(e) => {
            tracing::error!(error = %e, "plan store lookup failed");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "plan store lookup failed");
        }
    };

    (
        StatusCode::OK,
        Json(json!({
            "planId": plan_id,
            "planHash": generated.hash,
            "namespace": generated.namespace,
            "operations": generated.operations,
            "ddl": generated.ddl,
            "warnings": generated.warnings,
            "noop": generated.noop,
            "expiresAt": epoch_to_iso8601(created_epoch + PLAN_TTL_SECS),
        })),
    )
        .into_response()
}
