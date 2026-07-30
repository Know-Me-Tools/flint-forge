//! `GET /schema/v1/tables/{schema}/{table}/ddl` — `CREATE TABLE` synthesis
//! for an existing table (FFS-001 §4.4), so a client can drive
//! `registerEntityFromSql` at runtime.
//!
//! Behind `require_provisioner` (plan.md D-P3): the SQL-level grants on
//! column metadata already include `anon`, which makes this HTTP gate the
//! recon boundary that actually matters. Path identifiers are validated
//! BEFORE any query.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use fdb_app::provision::synthesize_create_table;
use fdb_domain::provision::Namespace;
use forge_domain::is_safe_identifier;
use serde_json::json;

use super::{error_response, require_enabled, require_provisioner, SchemaApiState};

/// Handler for `GET /schema/v1/tables/{schema}/{table}/ddl`.
pub async fn table_ddl(
    State(state): State<SchemaApiState>,
    Path((schema, table)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let provisioner = match require_enabled(&state) {
        Ok(p) => p,
        Err(resp) => return *resp,
    };
    if let Err(resp) = require_provisioner(&headers).await {
        return resp;
    }

    // Fail closed on injection-shaped path segments before touching the
    // database (they are bound, not interpolated — this is defense in depth
    // and produces a clearer 400 than a no-row lookup would).
    if schema.contains('.') || !is_safe_identifier(&schema) || table.contains('.')
        || !is_safe_identifier(&table)
    {
        return error_response(StatusCode::BAD_REQUEST, "invalid schema or table identifier");
    }

    let ns = Namespace(schema.clone());
    let info = match provisioner.table_ddl_info(&ns, &table).await {
        Ok(Some(info)) => info,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "unknown table"),
        Err(e) => {
            tracing::error!(namespace = %ns, error = %e, "ddl-info query failed");
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "ddl introspection failed");
        }
    };

    (
        StatusCode::OK,
        Json(json!({
            "schema": schema,
            "table": table,
            "ddl": synthesize_create_table(&table, &info),
            "rlsEnabled": info.rls_enabled,
            "rlsForced": info.rls_forced,
            "schemaVersion": state.state_manager.current().version,
        })),
    )
        .into_response()
}
