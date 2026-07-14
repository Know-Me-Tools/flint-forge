//! GraphQL query/mutation HTTP handler (POST /graphql).
//!
//! Relocated from `main.rs` (p16 file-size split) — behavior unchanged.
//! WebSocket subscription handling lives in `crate::subscriptions`.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use serde_json::json;

use fdb_app::graphql::introspection::{is_introspection_query, IntrospectionMerger};
use fdb_auth::rls_from_bearer;
use fdb_domain::GraphQlRequest;
use fdb_ports::GraphQlExecutor;

use crate::GatewayState;

/// GraphQL request body as sent by clients (queries and mutations only — subscriptions
/// use the WebSocket path).
#[derive(Debug, Deserialize)]
pub(crate) struct GraphQlBody {
    query: String,
    #[serde(default)]
    variables: serde_json::Value,
    #[serde(rename = "operationName")]
    operation_name: Option<String>,
}

/// POST /graphql — GraphQL query and mutation handler.
///
/// Extracts the bearer token from the `Authorization` header, builds `RlsContext`,
/// and delegates to `graphql.resolve()` via `PgGraphQl::execute()`.
/// The response is the raw pg_graphql JSON — no envelope added.
#[tracing::instrument(skip(state, headers, body), fields(operation_name = ?body.operation_name))]
pub(crate) async fn handle_graphql_query(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(body): Json<GraphQlBody>,
) -> impl IntoResponse {
    let Some(bearer) = extract_bearer(&headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"errors": [{"message": "missing Authorization header"}]})),
        )
            .into_response();
    };

    let rls = match rls_from_bearer(&bearer).await {
        Ok(ctx) => ctx,
        Err(e) => {
            tracing::warn!(error = %e, "bearer verification failed");
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"errors": [{"message": "invalid or expired token"}]})),
            )
                .into_response();
        }
    };

    let req = GraphQlRequest {
        query: body.query,
        variables: body.variables,
        operation_name: body.operation_name,
    };

    let is_introspection = is_introspection_query(&req.query);

    match state.graphql_executor.execute(req, &rls).await {
        Ok(mut result) => {
            // Merge subscription types into introspection responses.
            if is_introspection {
                let compiled = state.state_manager.current();
                if let Some(sub_schema) = compiled.subscription_schema.as_ref() {
                    result = IntrospectionMerger::merge(result, sub_schema);
                }
            }
            Json(result).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "graphql execution error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"errors": [{"message": "internal server error"}]})),
            )
                .into_response()
        }
    }
}

/// Extract the raw bearer token from the `Authorization: Bearer <token>` header.
/// Returns `None` if the header is absent or malformed.
pub(crate) fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(ToOwned::to_owned)
}
