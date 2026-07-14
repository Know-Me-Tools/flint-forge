//! Shared response builders and value-binding helpers used across the REST
//! compiler's handlers (list, mutations, rpc).
//!
//! Split out of `rest/mod.rs` to keep files under the 500-line limit.

use axum::{
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::compilers::filters::parse_filter_tree;

/// Keto namespace used for coarse relationship checks on table mutations.
pub(super) const KETO_NAMESPACE: &str = "entities";

/// Parse the non-reserved query params into an `fdb_query::FilterTree`, or return
/// a `400` response. Reserved keys are skipped inside the bridge.
pub(super) fn parse_filters(
    params: &HashMap<String, String>,
) -> Result<fdb_query::FilterTree, Box<axum::response::Response>> {
    parse_filter_tree(params).map_err(|e| Box::new(bad_request(&e.to_string())))
}

/// Bind a JSON body value as an uncast `$n` parameter, letting Postgres infer
/// the placeholder's type from the INSERT/UPDATE target column.
///
/// A `Value::String` binds as `QueryParam::Text` — matching Postgres's
/// inference for the common case of a `text`/`varchar` target column, so no
/// cast is needed or wanted. Casting `$n::jsonb` here was tried and is wrong:
/// Postgres's `jsonb → text` assignment cast preserves the JSON
/// representation (quotes included, e.g. `"tenant-a"` instead of
/// `tenant-a`), which silently corrupts string values and made every insert
/// whose value happened to also be compared by an RLS `WITH CHECK` policy
/// fail with "new row violates row-level security policy" — the value never
/// matched the unquoted comparison, discovered running this change's own
/// live-Postgres gate test. Non-string values still bind as
/// `QueryParam::Json` (uncast) for a genuinely `jsonb`-typed target column;
/// binding into a *typed, non-text, non-jsonb* column (`int4`, `bool`, `uuid`,
/// …) remains a known, separately-tracked gap (see this change's proposal.md
/// §3) since Postgres would infer that column's own type and neither
/// `Text` nor `Json` accepts it.
pub(super) fn json_bind(v: &Value) -> fdb_query::QueryParam {
    match v {
        Value::String(s) => fdb_query::QueryParam::Text(s.clone()),
        other => fdb_query::QueryParam::Json(other.to_string()),
    }
}

/// `201 Created` response for an insert, with a `Location` header pointing at
/// the new row. The primary-key value is read from the returned row when a
/// single-column `id` is present; otherwise `Location` targets the collection.
pub(super) fn insert_response(
    row: &serde_json::Map<String, Value>,
    schema: &str,
    table: &str,
) -> axum::response::Response {
    let body: Value = row.get("row").cloned().unwrap_or(Value::Null);
    let location = body
        .get("id")
        .map(|id| format!("/{schema}/{table}/{}", value_to_path(id)))
        .unwrap_or_else(|| format!("/{schema}/{table}"));

    (
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        Json(body),
    )
        .into_response()
}

/// `200 OK` response wrapping a JSON array of returned rows.
pub(super) fn rows_response(rows: &[serde_json::Map<String, Value>]) -> axum::response::Response {
    let out: Vec<Value> = rows
        .iter()
        .filter_map(|r| r.get("row").cloned())
        .collect();
    (StatusCode::OK, Json(Value::Array(out))).into_response()
}

/// Render a JSON scalar for use in a URL path segment.
pub(super) fn value_to_path(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// `403 Forbidden` — a mutation gate (Keto or Cedar) denied the request.
/// The body carries no subject, claim, or relation detail.
pub(super) fn forbidden() -> axum::response::Response {
    (StatusCode::FORBIDDEN, Json(json!({ "error": "forbidden" }))).into_response()
}

/// `400 Bad Request` with a JSON error body.
pub(super) fn bad_request(msg: &str) -> axum::response::Response {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))).into_response()
}

/// `500 Internal Server Error` with a generic body (never leaks DB detail).
pub(super) fn internal_error() -> axum::response::Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "internal server error" })),
    )
        .into_response()
}
