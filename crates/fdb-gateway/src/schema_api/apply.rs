//! `POST /schema/v1/apply` — idempotent apply with the D2 drift guard
//! (FFS-001 §4.3).
//!
//! Load the stored spec by hash (404/410) → re-plan against the *current*
//! live schema → refuse on hash mismatch (409: the hash covers spec + DDL,
//! so any live drift changes it) → adapter apply attributed to the caller's
//! JWT `sub` → honest response: `restartRequired: true` until the route
//! catch-all delegate exists (FFS-001 D8).

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use fdb_app::provision::generate;
use fdb_domain::provision::{PlanHash, ValidatedPlan};
use serde::Deserialize;
use serde_json::json;

use super::{
    error_response, now_epoch, require_allowlisted, require_enabled, require_provisioner,
    subject_of, SchemaApiState, PLAN_TTL_SECS,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ApplyRequest {
    plan_hash: PlanHash,
}

/// The D8 disclosure carried by every apply response while REST routes are
/// mounted once at startup.
const RESTART_NOTE: &str = "REST routes for new tables are mounted at startup; OpenAPI, MCP \
     tools, GraphQL subscriptions and the A2UI catalog are live now.";

/// Handler for `POST /schema/v1/apply`.
#[allow(clippy::too_many_lines)] // one linear request flow; splitting would obscure the state machine
pub async fn apply(
    State(state): State<SchemaApiState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let provisioner = match require_enabled(&state) {
        Ok(p) => p,
        Err(resp) => return *resp,
    };
    let caller = match require_provisioner(&headers).await {
        Ok(ctx) => ctx,
        Err(resp) => return resp,
    };

    let request: ApplyRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("invalid request: {e}")),
    };

    let stored = match provisioner.load_planned(&request.plan_hash).await {
        Ok(Some(stored)) => stored,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "unknown planHash"),
        Err(e) => {
            tracing::error!(error = %e, "plan store lookup failed");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "plan store lookup failed");
        }
    };

    let version_before = i64::try_from(state.state_manager.current().version).ok();

    if stored.status == "applied" {
        return (
            StatusCode::OK,
            Json(json!({
                "planId": stored.plan_id,
                "applied": true,
                "alreadyApplied": true,
                "schemaVersionBefore": version_before,
                "schemaVersionAfter": version_before,
                "reflectionRefreshed": false,
                "restartRequired": true,
                "restartNote": RESTART_NOTE,
            })),
        )
            .into_response();
    }
    if stored.status == "failed" {
        return error_response(
            StatusCode::CONFLICT,
            "previous apply of this plan failed (see ledger); create a new plan",
        );
    }
    if now_epoch() - stored.created_at_epoch >= PLAN_TTL_SECS {
        return error_response(StatusCode::GONE, "plan expired; re-plan and review again");
    }
    if let Err(resp) = require_allowlisted(&state, stored.spec.namespace.as_str()) {
        // The allowlist may have shrunk since plan time; apply must honor the
        // operator's *current* configuration.
        return *resp;
    }

    // Drift guard: re-plan the stored spec against the live schema and
    // compare the recomputed hash with the requested one.
    let live = match provisioner.introspect_namespace(&stored.spec.namespace).await {
        Ok(live) => live,
        Err(e) => {
            tracing::error!(namespace = %stored.spec.namespace, error = %e, "introspection failed");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "introspection failed");
        }
    };
    let regenerated = match generate(&stored.spec, &live) {
        Ok(plan) => plan,
        Err(e) => return error_response(StatusCode::CONFLICT, &format!(
            "plan no longer generates against the live schema: {e}"
        )),
    };
    if regenerated.hash != request.plan_hash {
        return error_response(
            StatusCode::CONFLICT,
            "plan no longer matches live schema",
        );
    }

    let validated = ValidatedPlan {
        plan_id: stored.plan_id.clone(),
        hash: regenerated.hash,
        namespace: regenerated.namespace,
        ddl: regenerated.ddl,
    };
    let applied = match provisioner
        .apply(&validated, &subject_of(&caller), version_before)
        .await
    {
        Ok(applied) => applied,
        Err(e) => {
            // The adapter's message is SQLSTATE-only by contract.
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
        }
    };

    // Sample the post-apply reflection version; the recompile is async
    // (event trigger → NOTIFY → StateManager), so an unchanged sample just
    // reports honestly and the ledger keeps version_after for later.
    let version_after = i64::try_from(state.state_manager.current().version).ok();
    let refreshed = version_after > version_before;
    if refreshed && !applied.already_applied {
        if let Some(after) = version_after {
            if let Err(e) = provisioner.record_version_after(&stored.plan_id, after).await {
                tracing::warn!(plan_id = %stored.plan_id, error = %e, "version_after not recorded");
            }
        }
    }

    (
        StatusCode::OK,
        Json(json!({
            "planId": stored.plan_id,
            "applied": true,
            "alreadyApplied": applied.already_applied,
            "schemaVersionBefore": version_before,
            "schemaVersionAfter": version_after,
            "reflectionRefreshed": refreshed,
            "restartRequired": true,
            "restartNote": RESTART_NOTE,
        })),
    )
        .into_response()
}
