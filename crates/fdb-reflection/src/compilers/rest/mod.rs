use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::{delete, get, patch, post},
};
use forge_domain::is_safe_identifier;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

use fdb_ports::KetoCheck;
use forge_policy::Pep;

mod mutations;
mod rpc;

use crate::compilers::filters::{bind_param, parse_filter_tree, render_where};
use crate::model::DatabaseModel;
use crate::passes::endpoint_generation::{EndpointKind, generate};

/// Default page size when no `Range` header is supplied (PostgREST-style cap).
const DEFAULT_LIMIT: i64 = 1000;

/// Keto namespace used for coarse relationship checks on table mutations.
pub(super) const KETO_NAMESPACE: &str = "entities";

/// Shared state threaded into every route handler.
///
/// `keto` and `pep` are optional gates. When present, mutation handlers call
/// them before executing SQL and return `403` on denial; when absent (early
/// boot, tests) the gates are skipped. Both trait objects are `Send + Sync`,
/// satisfying the web-domain requirement that handler state be shareable across
/// worker threads.
#[derive(Clone)]
pub(super) struct RestState {
    pub(super) model: Arc<DatabaseModel>,
    pub(super) pool: PgPool,
    pub(super) keto: Option<Arc<dyn KetoCheck>>,
    pub(super) pep: Option<Arc<dyn Pep>>,
}

/// Compiles a `DatabaseModel` into an Axum `Router` with CRUD + RPC handlers.
///
/// The resulting router exposes:
/// - `GET  /<schema>/<table>`       — list rows
/// - `POST /<schema>/<table>`       — insert row
/// - `PATCH /<schema>/<table>/:id`  — update row
/// - `DELETE /<schema>/<table>/:id` — delete row
/// - `POST /rpc/<schema>/<fn>`      — call stored function (vector args supported)
///
/// CRUD handlers remain `todo!()` stubs pending the query-builder landing.
/// `handle_rpc` is fully implemented: it detects `vector(N)` arg types and binds
/// `pgvector::Vector` typed parameters automatically.
pub struct RestCompiler;

impl RestCompiler {
    /// Compile without authorization gates (early boot / tests). Mutations run
    /// with RLS only. Prefer [`RestCompiler::compile_with_gates`] in production.
    pub fn compile(model: &DatabaseModel, pool: PgPool) -> Router {
        Self::compile_with_gates(model, pool, None, None)
    }

    /// Compile with optional Keto (coarse relationship) and Cedar (capability)
    /// gates. When supplied, every mutation handler calls both before touching
    /// SQL and returns `403` on denial.
    pub fn compile_with_gates(
        model: &DatabaseModel,
        pool: PgPool,
        keto: Option<Arc<dyn KetoCheck>>,
        pep: Option<Arc<dyn Pep>>,
    ) -> Router {
        let state = RestState {
            model: Arc::new(model.clone()),
            pool,
            keto,
            pep,
        };

        let endpoints = generate(model);

        let mut router: Router<RestState> = Router::new();

        for endpoint in &endpoints {
            let path = endpoint.path.clone();
            router = match (&endpoint.kind, endpoint.method) {
                (EndpointKind::TableList { .. }, "GET") => {
                    router.route(&path, get(handle_list))
                }
                (EndpointKind::TableList { .. }, "POST") => {
                    router.route(&path, post(mutations::handle_insert))
                }
                (EndpointKind::TableById { .. }, "PATCH") => {
                    router.route(&path, patch(mutations::handle_update))
                }
                (EndpointKind::TableById { .. }, "DELETE") => {
                    router.route(&path, delete(mutations::handle_delete))
                }
                (EndpointKind::Rpc { .. }, "POST") => {
                    router.route(&path, post(rpc::handle_rpc))
                }
                _ => router,
            };
        }

        router.with_state(state)
    }
}

/// `GET /<schema>/<table>` — list rows under the caller's RLS context.
///
/// Query params are PostgREST-style filters (`?col=eq.value`) except for the
/// reserved keys in [`RESERVED_PARAMS`]. Pagination is driven by the `Range`
/// header (`rows=<start>-<end>`); a `Content-Range` header echoes the served
/// window and total. RLS is enforced by the connection's GUC context — this
/// handler adds no extra GUC work.
///
/// SECURITY: schema, table, and every filter column pass through
/// [`is_safe_identifier`]; all values are bound as `$n`. No user-supplied
/// string is interpolated into SQL.
#[instrument(skip(state, params, headers), fields(schema = %schema, table = %table))]
async fn handle_list(
    State(state): State<RestState>,
    Path((schema, table)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !is_safe_identifier(&schema) || !is_safe_identifier(&table) {
        return bad_request("invalid schema or table identifier");
    }

    let filter_tree = match parse_filters(&params) {
        Ok(f) => f,
        Err(resp) => return *resp,
    };

    let (offset, limit) = parse_range(&headers);
    let where_clause = match render_where(&filter_tree, 1) {
        Ok(wc) => wc,
        Err(msg) => return bad_request(&msg),
    };
    // LIMIT/OFFSET placeholders follow the filter binds.
    let limit_idx = where_clause.binds.len() + 1;
    let offset_idx = where_clause.binds.len() + 2;

    let sql = format!(
        "SELECT COALESCE(json_agg(t), '[]'::json) AS rows, \
                (SELECT count(*) FROM {schema}.{table}) AS total \
         FROM (SELECT * FROM {schema}.{table} {where_sql} \
               ORDER BY 1 LIMIT ${limit_idx} OFFSET ${offset_idx}) t",
        where_sql = where_clause.sql,
    );

    let mut q = sqlx::query(&sql);
    for bind in &where_clause.binds {
        q = bind_param(q, bind);
    }
    q = q.bind(limit).bind(offset);

    match q.fetch_one(&state.pool).await {
        Ok(row) => list_response(&row, offset, limit),
        Err(e) => {
            tracing::error!(error = %e, "handle_list query error");
            internal_error()
        }
    }
}

/// Parse a `Range: rows=<start>-<end>` header into `(offset, limit)`.
///
/// Missing/malformed headers fall back to `(0, DEFAULT_LIMIT)`. An open-ended
/// range (`rows=10-`) uses the default limit from `start`.
fn parse_range(headers: &HeaderMap) -> (i64, i64) {
    let default = (0_i64, DEFAULT_LIMIT);
    let Some(val) = headers.get(header::RANGE).and_then(|v| v.to_str().ok()) else {
        return default;
    };
    let Some(spec) = val.trim().strip_prefix("rows=") else {
        return default;
    };
    let Some((start_s, end_s)) = spec.split_once('-') else {
        return default;
    };
    let Ok(start) = start_s.trim().parse::<i64>() else {
        return default;
    };
    if start < 0 {
        return default;
    }
    match end_s.trim().parse::<i64>() {
        Ok(end) if end >= start => (start, end - start + 1),
        _ => (start, DEFAULT_LIMIT),
    }
}

/// Build the `200 OK` list response with a `Content-Range` header.
fn list_response(row: &sqlx::postgres::PgRow, offset: i64, limit: i64) -> axum::response::Response {
    use sqlx::Row;
    let rows: Value = row.try_get("rows").unwrap_or(Value::Array(vec![]));
    let total: i64 = row.try_get("total").unwrap_or(0);

    let count = rows.as_array().map_or(0, Vec::len) as i64;
    let start = offset;
    // `end` is the index of the last returned row (inclusive); -1 when empty.
    let end = if count == 0 { start } else { start + count - 1 };
    let content_range = format!("rows {start}-{end}/{total}");
    let _ = limit; // limit shaped the query; the window is described by count.

    (
        StatusCode::OK,
        [(header::CONTENT_RANGE, content_range)],
        Json(rows),
    )
        .into_response()
}

/// Parse the non-reserved query params into an `fdb_query::FilterTree`, or return
/// a `400` response. Reserved keys are skipped inside the bridge.
pub(super) fn parse_filters(
    params: &HashMap<String, String>,
) -> Result<fdb_query::FilterTree, Box<axum::response::Response>> {
    parse_filter_tree(params).map_err(|e| Box::new(bad_request(&e.to_string())))
}

/// Bind a JSON value as a Postgres parameter.
///
/// Strings bind as text; everything else binds as JSONB (`Value`), letting
/// Postgres coerce to the target column type. This keeps the value on the
/// parameter channel — it is never interpolated into SQL.
pub(super) fn json_bind(v: &Value) -> Value {
    v.clone()
}

/// `201 Created` response for an insert, with a `Location` header pointing at
/// the new row. The primary-key value is read from the returned row when a
/// single-column `id` is present; otherwise `Location` targets the collection.
pub(super) fn insert_response(
    row: &sqlx::postgres::PgRow,
    schema: &str,
    table: &str,
) -> axum::response::Response {
    use sqlx::Row;
    let body: Value = row.try_get("row").unwrap_or(Value::Null);
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
pub(super) fn rows_response(rows: &[sqlx::postgres::PgRow]) -> axum::response::Response {
    use sqlx::Row;
    let out: Vec<Value> = rows
        .iter()
        .filter_map(|r| r.try_get::<Value, _>("row").ok())
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


#[cfg(test)]
mod tests {
    use crate::model::{DatabaseModel, Table};
    use super::RestCompiler;

    fn minimal_model() -> DatabaseModel {
        DatabaseModel {
            tables: vec![Table {
                schema: "public".into(),
                name: "items".into(),
                columns: vec![],
                pk: vec![],
                fk: vec![],
                rls_enabled: true,
                vault_key: None,
            }],
            functions: vec![],
            views: vec![],
            version: 1,
        }
    }

    #[tokio::test]
    async fn compiles_without_panic_for_minimal_model() {
        // compile() must not panic during route registration.
        // Uses a disconnected lazy pool — compilation never touches the DB.
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test")
            .expect("lazy pool");
        let model = minimal_model();
        let _router = RestCompiler::compile(&model, pool);
    }

    // json_to_vector tests live with the handler in `rpc.rs`.

    fn range_header(val: &str) -> axum::http::HeaderMap {
        let mut h = axum::http::HeaderMap::new();
        h.insert(axum::http::header::RANGE, val.parse().unwrap());
        h
    }

    #[test]
    fn parse_range_reads_closed_range() {
        use super::parse_range;
        // rows=0-9 → offset 0, limit 10
        assert_eq!(parse_range(&range_header("rows=0-9")), (0, 10));
        // rows=10-19 → offset 10, limit 10
        assert_eq!(parse_range(&range_header("rows=10-19")), (10, 10));
    }

    #[test]
    fn parse_range_defaults_when_absent_or_malformed() {
        use super::{parse_range, DEFAULT_LIMIT};
        use axum::http::HeaderMap;
        assert_eq!(parse_range(&HeaderMap::new()), (0, DEFAULT_LIMIT));
        assert_eq!(parse_range(&range_header("items=0-9")), (0, DEFAULT_LIMIT));
        assert_eq!(parse_range(&range_header("rows=abc")), (0, DEFAULT_LIMIT));
        // negative start is rejected
        assert_eq!(parse_range(&range_header("rows=-5-9")), (0, DEFAULT_LIMIT));
    }

    #[test]
    fn parse_range_open_ended_uses_default_limit() {
        use super::{parse_range, DEFAULT_LIMIT};
        // rows=20- → offset 20, default limit
        assert_eq!(parse_range(&range_header("rows=20-")), (20, DEFAULT_LIMIT));
    }

}
