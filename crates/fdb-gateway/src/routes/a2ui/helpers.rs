//! Shared helpers for A2UI route handlers (claims extraction and error
//! mapping). Kept separate so every handler module can depend on them
//! without duplicating logic.

use axum::{http::StatusCode, response::Json};
use forge_identity::RlsContext;
use serde_json::{json, Value};

/// Build a JSON object from the `RlsContext` claims string.
pub(super) fn claims_json(who: &RlsContext) -> Value {
    let mut claims: Value = serde_json::from_str(&who.claims_json).unwrap_or(Value::Null);
    if claims.pointer("/flint/user_id").is_none() {
        if let Some(subject) = claims.get("sub").and_then(Value::as_str).map(String::from) {
            if let Some(object) = claims.as_object_mut() {
                let flint = object.entry("flint").or_insert_with(|| json!({}));
                if let Some(flint) = flint.as_object_mut() {
                    flint.insert("user_id".into(), Value::String(subject));
                }
            }
        }
    }
    claims
}

pub(super) fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, Json<Value>) {
    tracing::error!(error = %err, "a2ui api error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "internal server error" })),
    )
}

/// Every catalog request uses one transaction with the verified identity.
/// Dropping the transaction resets both role and claims before pool reuse.
pub(super) async fn transaction(
    pool: &sqlx::PgPool,
    who: &RlsContext,
) -> Result<sqlx::Transaction<'static, sqlx::Postgres>, (StatusCode, Json<Value>)> {
    let mut tx = pool.begin().await.map_err(internal_error)?;
    sqlx::query("SELECT set_config('role', $1, true), set_config('request.jwt.claims', $2, true), set_config('app.jwt_claims', $2, true), set_config('request.headers', $3, true)")
        .bind(&who.role)
        .bind(claims_json(who).to_string())
        .bind(json!({"authorization": format!("Bearer {}", who.raw_bearer)}).to_string())
        .execute(&mut *tx).await.map_err(internal_error)?;
    Ok(tx)
}
