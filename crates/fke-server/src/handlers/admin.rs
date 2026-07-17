//! Control-plane (feature=control-plane): register + list functions.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use forge_identity::RlsContext;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::handlers::invoke::extract_bearer;
use crate::state::{verify_manifest_signature, KilnState};

/// The Postgres/JWT role this repo treats as privileged everywhere else
/// (`ext-flint-meta`, `fdb-gateway`'s policy source — see their doc comments)
/// — reused here so `/admin/functions` matches the same admin convention
/// rather than inventing a Kiln-specific one.
const ADMIN_ROLE: &str = "service_role";

/// p16-c003: require a valid bearer AND an admin-scoped role for
/// `/admin/functions` — previously this route had no auth middleware at all,
/// gated only by the compile-time `control-plane` feature flag (which
/// controls whether the route is mounted, not who may call it).
pub(crate) async fn require_admin(
    headers: &HeaderMap,
) -> Result<RlsContext, axum::response::Response> {
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
pub(crate) struct RegisterBody {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) manifest: fke_domain::FunctionManifest,
    /// Raw WASM component bytes (base64-encoded).
    pub(crate) wasm_base64: String,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
struct RegisterResponse {
    function_id: String,
    content_digest: String,
}

#[allow(unused_variables)]
pub(crate) async fn register_function(
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

pub(crate) async fn list_functions(
    State(state): State<KilnState>,
    headers: HeaderMap,
) -> impl IntoResponse {
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
