//! Application listing/detail and design-system token handlers.
//!
//! - `GET    /a2ui/v1/applications`
//! - `GET    /a2ui/v1/applications/{id}`
//! - `GET    /a2ui/v1/design-systems/{id}/tokens`

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    Extension,
};
use forge_identity::RlsContext;
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::{types::Json as SqlxJson, FromRow};
use uuid::Uuid;

use super::helpers::{internal_error, user_id_from_claims};
use super::A2uiState;

/// Application row.
#[derive(Debug, Serialize, FromRow)]
struct ApplicationRow {
    id: Uuid,
    slug: String,
    name: String,
    description: Option<String>,
    jwt_claims_template: SqlxJson<Value>,
    catalog_id: Option<String>,
    is_system: bool,
}

/// `GET /a2ui/v1/applications`
///
/// Lists applications the caller has access to. System applications are always
/// visible; non-system applications require a role assignment.
pub async fn list_applications(
    State(state): State<A2uiState>,
    Extension(who): Extension<RlsContext>,
) -> impl IntoResponse {
    let user_id = user_id_from_claims(&who);

    let apps: Vec<ApplicationRow> = sqlx::query_as(
        "SELECT a.id, a.slug, a.name, a.description, a.jwt_claims_template, a.catalog_id, a.is_system
         FROM flint_a2ui.applications a
         WHERE a.is_system = true
            OR $1::text IS NULL
            OR a.id IN (
                SELECT DISTINCT application_id FROM flint_a2ui.role_assignments
                WHERE user_id = $1
            )
         ORDER BY a.is_system DESC, a.slug",
    )
    .bind(user_id.as_deref())
    .fetch_all(&state.pool)
    .await
    .map_err(internal_error)?;

    Ok::<_, (StatusCode, Json<Value>)>(Json(json!({ "applications": apps })))
}

/// `GET /a2ui/v1/applications/{id}`
///
/// Returns a single application.
pub async fn get_application(
    State(state): State<A2uiState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let app: Option<ApplicationRow> = sqlx::query_as(
        "SELECT id, slug, name, description, jwt_claims_template, catalog_id, is_system
         FROM flint_a2ui.applications WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?;

    match app {
        Some(a) => Ok(Json(json!({ "application": a }))),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "application not found"})),
        )),
    }
}

/// `GET /a2ui/v1/design-systems/{id}/tokens`
///
/// Returns the design system's tokens in W3C Design Token format.
/// The tokens are stored as jsonb in `flint_a2ui.design_systems.tokens`.
pub async fn get_design_system_tokens(
    State(state): State<A2uiState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let row: Option<(SqlxJson<Value>,)> =
        sqlx::query_as("SELECT tokens FROM flint_a2ui.design_systems WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.pool)
            .await
            .map_err(internal_error)?;

    match row {
        Some((tokens,)) => Ok(Json(tokens.0)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "design system not found"})),
        )),
    }
}
