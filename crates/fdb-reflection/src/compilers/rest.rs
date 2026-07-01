use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::{delete, get, patch, post},
};
use forge_domain::is_safe_identifier;
use serde_json::{Map, Value, json};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

use crate::compilers::filters::{
    Filter, RESERVED_PARAMS, build_where, parse_filter,
};
use crate::model::{DatabaseModel, is_vector_type};
use crate::passes::endpoint_generation::{EndpointKind, generate};

/// Default page size when no `Range` header is supplied (PostgREST-style cap).
const DEFAULT_LIMIT: i64 = 1000;

/// Shared state threaded into every route handler.
#[derive(Clone)]
struct RestState {
    model: Arc<DatabaseModel>,
    pool: PgPool,
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
    pub fn compile(model: &DatabaseModel, pool: PgPool) -> Router {
        let state = RestState {
            model: Arc::new(model.clone()),
            pool,
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
                    router.route(&path, post(handle_insert))
                }
                (EndpointKind::TableById { .. }, "PATCH") => {
                    router.route(&path, patch(handle_update))
                }
                (EndpointKind::TableById { .. }, "DELETE") => {
                    router.route(&path, delete(handle_delete))
                }
                (EndpointKind::Rpc { .. }, "POST") => {
                    router.route(&path, post(handle_rpc))
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

    // Parse filters (skip reserved param keys).
    let mut filters: Vec<Filter> = Vec::new();
    for (key, raw) in &params {
        if RESERVED_PARAMS.contains(&key.as_str()) {
            continue;
        }
        match parse_filter(key, raw) {
            Ok(f) => filters.push(f),
            Err(e) => return bad_request(&e.to_string()),
        }
    }

    let (offset, limit) = parse_range(&headers);
    let where_clause = build_where(&filters, 1);
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
        q = q.bind(bind);
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

/// `400 Bad Request` with a JSON error body.
fn bad_request(msg: &str) -> axum::response::Response {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))).into_response()
}

/// `500 Internal Server Error` with a generic body (never leaks DB detail).
fn internal_error() -> axum::response::Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "internal server error" })),
    )
        .into_response()
}

async fn handle_insert(State(_state): State<RestState>) -> StatusCode {
    todo!("p2-c004: insert row via REST query builder")
}

async fn handle_update(
    State(_state): State<RestState>,
    Path(_id): Path<String>,
) -> StatusCode {
    todo!("p2-c004: update row via REST query builder")
}

async fn handle_delete(
    State(_state): State<RestState>,
    Path(_id): Path<String>,
) -> StatusCode {
    todo!("p2-c004: delete row via REST query builder")
}

/// POST /rpc/<schema>/<fn_name> — call a Postgres function with optional vector args.
///
/// Accepts a JSON object body where keys are arg names and values are arg values.
/// Args whose `pg_type` starts with `"vector"` are deserialized from `[f32, ...]`
/// JSON arrays into `pgvector::Vector` and bound with explicit `::vector` casting.
/// All other args are bound as generic JSONB.
///
/// Returns the result rows as a JSON array. Returns 400 on malformed vector args
/// or unknown function, 500 on database errors.
#[instrument(skip(state, body), fields(schema = %schema, fn_name = %fn_name))]
async fn handle_rpc(
    State(state): State<RestState>,
    Path((schema, fn_name)): Path<(String, String)>,
    Json(body): Json<Map<String, Value>>,
) -> impl IntoResponse {
    // Locate the function metadata in the compiled model.
    let fn_meta = match state
        .model
        .functions
        .iter()
        .find(|f| f.schema == schema && f.name == fn_name)
    {
        Some(f) => f,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": format!("function {schema}.{fn_name} not found")})),
            )
                .into_response();
        }
    };

    // Build the parameterised SQL call:
    //   SELECT * FROM <schema>.<fn>($1, $2, ...) AS t
    // Placeholder numbering matches the arg order from FnMeta.
    let placeholders: Vec<String> = (1..=fn_meta.args.len())
        .map(|i| format!("${i}"))
        .collect();
    // SECURITY: interpolate the identifiers from the *compiled model*
    // (`fn_meta`), not the raw path params — the model is reflected from
    // pg_catalog and is the trusted source. Belt-and-braces, re-validate.
    if !is_safe_identifier(&fn_meta.schema) || !is_safe_identifier(&fn_meta.name) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "invalid function identifier"})),
        )
            .into_response();
    }
    let call_sql = format!(
        "SELECT row_to_json(t) AS row FROM {}.{}({}) AS t",
        fn_meta.schema,
        fn_meta.name,
        placeholders.join(", ")
    );

    // Bind arguments in declaration order, dispatching on pg_type.
    let mut q = sqlx::query(&call_sql);
    for arg in &fn_meta.args {
        let val = body.get(&arg.name).cloned().unwrap_or(Value::Null);
        if is_vector_type(&arg.pg_type) {
            match json_to_vector(val) {
                Ok(vec) => {
                    q = q.bind(vec);
                }
                Err(msg) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error": msg})),
                    )
                        .into_response();
                }
            }
        } else {
            q = q.bind(val);
        }
    }

    // Execute and collect result rows.
    match q.fetch_all(&state.pool).await {
        Ok(rows) => {
            let result: Vec<Value> = rows
                .into_iter()
                .filter_map(|row| {
                    use sqlx::Row;
                    row.try_get::<Value, _>("row").ok()
                })
                .collect();
            Json(Value::Array(result)).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, schema = %schema, fn_name = %fn_name, "rpc execution error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal server error"})),
            )
                .into_response()
        }
    }
}

/// Deserialize a JSON value as a `pgvector::Vector`.
///
/// Expects a JSON array of numbers. Returns a descriptive error string on mismatch.
fn json_to_vector(val: Value) -> Result<pgvector::Vector, String> {
    let arr = match val {
        Value::Array(a) => a,
        Value::Null => return Err("vector arg must be a JSON array, got null".into()),
        other => {
            return Err(format!(
                "vector arg must be a JSON array, got {}",
                other.type_name_for_error()
            ));
        }
    };

    let floats: Result<Vec<f32>, _> = arr
        .into_iter()
        .map(|v| {
            v.as_f64()
                .map(|f| f as f32)
                .ok_or_else(|| "vector elements must be numbers".to_owned())
        })
        .collect();

    Ok(pgvector::Vector::from(floats?))
}

/// Helper to name the JSON type in error messages.
trait JsonTypeName {
    fn type_name_for_error(&self) -> &'static str;
}

impl JsonTypeName for Value {
    fn type_name_for_error(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }
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

    #[test]
    fn json_to_vector_accepts_float_array() {
        use serde_json::json;
        use super::json_to_vector;
        let v = json!([0.1_f64, 0.2_f64, 0.3_f64]);
        let vec = json_to_vector(v).expect("should parse");
        let floats: Vec<f32> = vec.into();
        assert_eq!(floats.len(), 3);
        assert!((floats[0] - 0.1_f32).abs() < 1e-6);
    }

    #[test]
    fn json_to_vector_rejects_non_array() {
        use serde_json::json;
        use super::json_to_vector;
        assert!(json_to_vector(json!("not an array")).is_err());
        assert!(json_to_vector(json!(42)).is_err());
        assert!(json_to_vector(serde_json::Value::Null).is_err());
    }

    #[test]
    fn json_to_vector_rejects_non_numeric_elements() {
        use serde_json::json;
        use super::json_to_vector;
        assert!(json_to_vector(json!(["a", "b"])).is_err());
    }

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
