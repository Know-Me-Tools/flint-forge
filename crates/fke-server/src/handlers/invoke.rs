//! Data-plane: `POST /functions/v1/{name}[@{version}]` — invoke a registered function.

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use fke_domain::ContentId;
use fke_runtime::KilnRequest;
use serde_json::json;

use crate::state::{verify_manifest_signature, KilnState};

/// Splits a `/functions/v1/{name_or_versioned}` path segment into
/// `(name, version)` — `"foo@1.2.3"` → `("foo", "1.2.3")`, `"foo"` →
/// `("foo", "latest")`. Extracted as a pure function so the axum 0.8
/// one-capture-per-segment workaround (see `invoke_function`) has
/// independent test coverage.
fn split_name_version(name_or_versioned: &str) -> (&str, &str) {
    match name_or_versioned.split_once('@') {
        Some((name, version)) => (name, version),
        None => (name_or_versioned, "latest"),
    }
}

/// Handles both `/functions/v1/{name}` and `/functions/v1/{name}@{version}`
/// — axum 0.8 permits only one dynamic capture per path segment, so both
/// forms are registered as a single route capturing the whole segment; the
/// optional `@{version}` suffix is split out here instead.
#[tracing::instrument(skip(state, headers, body), fields(function = %name_or_versioned))]
pub(crate) async fn invoke_function(
    State(state): State<KilnState>,
    Path(name_or_versioned): Path<String>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let (name, version) = split_name_version(&name_or_versioned);
    invoke_impl(&state, name, version, headers, body).await
}

pub(crate) async fn invoke_impl(
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
    if let Err(resp) =
        ensure_loaded_and_verified(state, &manifest, &content_id, name, version).await
    {
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

    if let Err(e) = state
        .runtime
        .load_wasm(content_id.clone(), &wasm_bytes, &manifest.capabilities)
    {
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
pub(crate) fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression for p16-c008: `/functions/v1/{name}@{version}` used to be
    /// its own route with two captures in one segment, which axum 0.8
    /// rejects at startup ("Only one parameter is allowed per path
    /// segment"), crash-looping fke-server before it ever bound a listener.
    #[test]
    fn split_name_version_extracts_both_parts_when_at_present() {
        assert_eq!(split_name_version("echo@1.2.3"), ("echo", "1.2.3"));
    }

    #[test]
    fn split_name_version_defaults_to_latest_when_at_absent() {
        assert_eq!(split_name_version("echo"), ("echo", "latest"));
    }

    #[test]
    fn split_name_version_splits_on_first_at_only() {
        assert_eq!(split_name_version("echo@1@2"), ("echo", "1@2"));
    }
}
