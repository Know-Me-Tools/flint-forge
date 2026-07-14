//! A2UI surface emission over the AG-UI event stream.

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Extension, Json,
};
use forge_identity::RlsContext;

use fdb_domain::AgUiEvent;

use super::state::AgUiState;

/// Emit an assembled A2UI surface as an AG-UI `Custom` event with
/// `name: "a2ui:surface"`. The payload follows the A2UI v0.9 wire format:
///
/// ```json
/// { "protocol": "a2ui/0.9", "messages": [ ... ] }
/// ```
///
/// Agents and frontends listen for `"a2ui:surface"` Custom events on the run
/// stream and render the surface using their registered catalog.
pub async fn emit_a2ui_surface(
    state: &AgUiState,
    run_id: &str,
    surface: &fdb_reflection::compilers::a2ui::A2uiSurface,
) {
    let payload = serde_json::json!({
        "protocol": "a2ui/0.9",
        "catalogId": surface.catalog_id,
        "messages": surface.messages,
    });
    state
        .publish(AgUiEvent::Custom {
            run_id: run_id.to_owned(),
            name: "a2ui:surface".to_owned(),
            value: payload,
        })
        .await;
}

/// Request body for `POST /agents/v1/:run_id/surfaces/assemble`.
#[derive(Debug, serde::Deserialize)]
pub struct AssembleSurfaceForRunBody {
    pub event_type: String,
    #[serde(default)]
    pub event_context: serde_json::Value,
    #[serde(default)]
    pub application_id: Option<uuid::Uuid>,
}

/// `POST /agents/v1/:run_id/surfaces/assemble`
///
/// Assembles an A2UI surface for the event context and emits it immediately
/// into the run's event stream as a `Custom` event with type `"a2ui:surface"`.
/// Frontends subscribed to the run's SSE stream receive the surface and render
/// it using the Flint catalog.
///
/// Cedar `a2ui:emit` capability gate: callers must be authenticated; the
/// assembled component is filtered by `flint_a2ui.resolve_components()` which
/// respects application role assignments.
pub async fn assemble_and_emit_surface(
    State(state): State<AgUiState>,
    Extension(who): Extension<RlsContext>,
    Path(run_id): Path<String>,
    Json(body): Json<AssembleSurfaceForRunBody>,
) -> impl IntoResponse {
    use fdb_reflection::compilers::a2ui::{A2uiAssembler, AssemblyContext};

    // Require the run to exist (channel must already be open).
    {
        let runs = state.inner.runs.lock().await;
        if !runs.contains_key(&run_id) {
            return (
                axum::http::StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "run not found"})),
            )
                .into_response();
        }
    }

    let claims: serde_json::Value =
        serde_json::from_str(&who.claims_json).unwrap_or(serde_json::Value::Null);
    let ctx = AssemblyContext {
        event_type: body.event_type,
        event_payload: body.event_context,
        application_id: body.application_id,
        jwt_claims: claims,
        surface_id: None,
    };

    let surface = match state
        .a2ui_pool
        .as_ref()
        .map(|pool| A2uiAssembler::new(pool.clone()))
    {
        Some(assembler) => match assembler.assemble(&ctx).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "A2UI assembly failed");
                return (
                    axum::http::StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": e.to_string()})),
                )
                    .into_response();
            }
        },
        None => {
            return (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "A2UI pool not configured"})),
            )
                .into_response();
        }
    };

    emit_a2ui_surface(&state, &run_id, &surface).await;

    Json(serde_json::json!({
        "status": "emitted",
        "run_id": run_id,
        "surface_id": surface.surface_id,
        "catalog_id": surface.catalog_id,
    }))
    .into_response()
}
