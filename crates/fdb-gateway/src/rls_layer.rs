//! RLS extraction middleware for the reflection router.
//!
//! Mutation handlers in `fdb-reflection` require an `Extension<RlsContext>` to
//! run the Keto + Cedar gates. This middleware verifies the bearer token, builds
//! the `RlsContext`, and inserts it into request extensions. Requests without a
//! valid token are rejected with `401` before reaching a handler.
//!
//! SECURITY: the bearer and claims are never logged. Verification failures are
//! logged at `warn` with the error code only.

use axum::{
    extract::Request,
    http::{header, HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;

use fdb_auth::rls_from_bearer;

/// Extract the `Bearer` token from an `Authorization` header, if present.
fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(str::to_owned)
}

/// Middleware: verify the bearer and attach `RlsContext` to request extensions.
pub async fn require_rls(mut req: Request, next: Next) -> Response {
    let Some(bearer) = extract_bearer(req.headers()) else {
        return unauthorized("missing Authorization header");
    };
    match rls_from_bearer(&bearer).await {
        Ok(ctx) => {
            req.extensions_mut().insert(ctx);
            next.run(req).await
        }
        Err(e) => {
            tracing::warn!(error = %e, "bearer verification failed");
            unauthorized("invalid or expired token")
        }
    }
}

fn unauthorized(msg: &str) -> Response {
    (StatusCode::UNAUTHORIZED, Json(json!({ "error": msg }))).into_response()
}
