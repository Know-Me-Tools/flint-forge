//! Surface assembly handler.
//!
//! - `POST   /a2ui/v1/surfaces/assemble`

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    Extension,
};
use fdb_reflection::compilers::a2ui::{A2uiAssembler, AssemblerError, AssemblyContext};
use forge_identity::RlsContext;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use super::helpers::claims_json;
use super::A2uiState;

/// JSON body for `POST /a2ui/v1/surfaces/assemble`.
#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct AssembleSurfaceBody {
    pub event_type: String,
    #[serde(default)]
    pub event_context: Value,
    #[serde(default)]
    pub application_id: Option<Uuid>,
}

/// `POST /a2ui/v1/surfaces/assemble`
///
/// Assembles an A2UI surface from an event context. Delegates to
/// `A2uiAssembler` in `fdb-reflection`, which applies application-specific
/// assembly rules and falls back to default table bindings.
#[tracing::instrument(skip(state, who, body), fields(event_type = %body.event_type))]
pub async fn assemble_surface(
    State(state): State<A2uiState>,
    Extension(who): Extension<RlsContext>,
    Json(body): Json<AssembleSurfaceBody>,
) -> impl IntoResponse {
    assemble_surface_value(&state.pool, &who, &body).await
}

/// Inner logic shared with the MCP tool.
pub async fn assemble_surface_value(
    pool: &sqlx::PgPool,
    who: &RlsContext,
    body: &AssembleSurfaceBody,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let ctx = AssemblyContext {
        event_type: body.event_type.clone(),
        event_payload: body.event_context.clone(),
        application_id: body.application_id,
        jwt_claims: claims_json(who),
        surface_id: None,
    };

    let assembler = A2uiAssembler::new(pool.clone());
    match assembler.assemble(&ctx).await {
        Ok(surface) => Ok(Json(surface.to_json())),
        Err(err) => Err(assembler_error(err)),
    }
}

fn assembler_error(err: AssemblerError) -> (StatusCode, Json<Value>) {
    match err {
        AssemblerError::NoBinding(schema, table) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "no binding",
                "schema": schema,
                "table": table,
            })),
        ),
        AssemblerError::MissingField(field) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "missing field",
                "field": field,
            })),
        ),
        AssemblerError::InvalidConfig(msg) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "invalid config",
                "message": msg,
            })),
        ),
        AssemblerError::Database(e) => {
            tracing::error!(error = %e, "a2ui assembler database error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "internal server error" })),
            )
        }
        _ => {
            tracing::error!(error = %err, "a2ui assembler error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "internal server error" })),
            )
        }
    }
}
