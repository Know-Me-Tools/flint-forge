use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Extension, Json, Router,
};
use forge_domain::is_safe_identifier;
use forge_identity::RlsContext;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

use fdb_ports::{KetoCheck, SqlExecutor};
use forge_policy::Pep;

mod mutations;
mod rpc;

use fdb_query::embed::{
    parse_embed_select, render_inner_guards, render_projection, resolve_embeds,
    route_embedded_param,
};
use fdb_query::QueryParam;

use crate::compilers::embed_schema::embed_schema_from_model;
use crate::compilers::filters::{parse_filter_tree, render_where};
use crate::model::DatabaseModel;
use crate::passes::endpoint_generation::{generate, EndpointKind};

/// Default page size when no `Range` header is supplied (PostgREST-style cap).
const DEFAULT_LIMIT: i64 = 1000;

/// Keto namespace used for coarse relationship checks on table mutations.
pub(super) const KETO_NAMESPACE: &str = "entities";

/// Shared state threaded into every route handler.
///
/// `executor` runs every CRUD/`rpc` SQL statement inside an RLS-scoped
/// transaction (`SqlExecutor::execute_raw` → `PgBackend::acquire(rls)`) — no
/// handler ever touches a raw, unscoped connection. `keto` and `pep` are
/// optional gates. When present, mutation handlers call them before executing
/// SQL and return `403` on denial; when absent (early boot, tests) the gates
/// are skipped. All fields are `Send + Sync`, satisfying the web-domain
/// requirement that handler state be shareable across worker threads.
#[derive(Clone)]
pub(super) struct RestState {
    pub(super) model: Arc<DatabaseModel>,
    pub(super) executor: Arc<dyn SqlExecutor>,
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
/// Every handler runs its SQL through `executor.execute_raw(sql, params, rls)`
/// (`SqlExecutor`, `fdb-ports`), which `acquire(rls)`s a connection and issues
/// the `SET LOCAL` GUCs before the statement runs — RLS is enforced on every
/// route this compiler mounts, not assumed from the pool's ambient role.
/// `handle_rpc` additionally detects `vector(N)` arg types and binds
/// `pgvector::Vector` typed parameters automatically.
pub struct RestCompiler;

impl RestCompiler {
    /// Compile without authorization gates (early boot / tests). Mutations run
    /// with RLS only. Prefer [`RestCompiler::compile_with_gates`] in production.
    pub fn compile(model: &DatabaseModel, executor: Arc<dyn SqlExecutor>) -> Router {
        Self::compile_with_gates(model, executor, None, None)
    }

    /// Compile with optional Keto (coarse relationship) and Cedar (capability)
    /// gates. When supplied, every mutation handler calls both before touching
    /// SQL and returns `403` on denial.
    pub fn compile_with_gates(
        model: &DatabaseModel,
        executor: Arc<dyn SqlExecutor>,
        keto: Option<Arc<dyn KetoCheck>>,
        pep: Option<Arc<dyn Pep>>,
    ) -> Router {
        let state = RestState {
            model: Arc::new(model.clone()),
            executor,
            keto,
            pep,
        };

        let endpoints = generate(model);

        let mut router: Router<RestState> = Router::new();

        // Every route below is a concrete, literal path — `schema`/`table` (or
        // `func`'s schema/name) are compile-time-known from the `DatabaseModel`,
        // not runtime path parameters (axum never captures them: there is no
        // `{schema}`/`{table}` segment in any registered path). Each handler
        // closure binds its own table/function identity via capture instead of
        // a `Path` extractor — the extractor approach silently mismatched (0
        // captured segments vs. 2 expected) and every CRUD/`rpc` request 500'd
        // before reaching a handler body. Discovered running this change's own
        // live-Postgres gate test.
        for endpoint in &endpoints {
            let path = endpoint.path.clone();
            router = match (&endpoint.kind, endpoint.method) {
                (EndpointKind::TableList { table }, "GET") => {
                    let schema = table.schema.clone();
                    let name = table.name.clone();
                    router.route(
                        &path,
                        get(move |State(state): State<RestState>,
                                  Extension(rls): Extension<RlsContext>,
                                  Query(params): Query<HashMap<String, String>>,
                                  headers: HeaderMap| {
                            let schema = schema.clone();
                            let name = name.clone();
                            async move {
                                handle_list(state, rls, schema, name, params, headers).await
                            }
                        }),
                    )
                }
                (EndpointKind::TableList { table }, "POST") => {
                    let schema = table.schema.clone();
                    let name = table.name.clone();
                    router.route(
                        &path,
                        post(move |State(state): State<RestState>,
                                   Extension(rls): Extension<RlsContext>,
                                   Json(body): Json<Map<String, Value>>| {
                            let schema = schema.clone();
                            let name = name.clone();
                            async move {
                                mutations::handle_insert(state, schema, name, rls, Json(body)).await
                            }
                        }),
                    )
                }
                (EndpointKind::TableById { table }, "PATCH") => {
                    let schema = table.schema.clone();
                    let name = table.name.clone();
                    router.route(
                        &path,
                        patch(move |State(state): State<RestState>,
                                    Extension(rls): Extension<RlsContext>,
                                    Query(params): Query<HashMap<String, String>>,
                                    Json(body): Json<Map<String, Value>>| {
                            let schema = schema.clone();
                            let name = name.clone();
                            async move {
                                mutations::handle_update(
                                    state,
                                    schema,
                                    name,
                                    rls,
                                    Query(params),
                                    Json(body),
                                )
                                .await
                            }
                        }),
                    )
                }
                (EndpointKind::TableById { table }, "DELETE") => {
                    let schema = table.schema.clone();
                    let name = table.name.clone();
                    router.route(
                        &path,
                        delete(move |State(state): State<RestState>,
                                     Extension(rls): Extension<RlsContext>,
                                     Query(params): Query<HashMap<String, String>>| {
                            let schema = schema.clone();
                            let name = name.clone();
                            async move {
                                mutations::handle_delete(state, schema, name, rls, Query(params))
                                    .await
                            }
                        }),
                    )
                }
                (EndpointKind::Rpc { func }, "POST") => {
                    let schema = func.schema.clone();
                    let name = func.name.clone();
                    router.route(
                        &path,
                        post(move |State(state): State<RestState>,
                                   Extension(rls): Extension<RlsContext>,
                                   Json(body): Json<Map<String, Value>>| {
                            let schema = schema.clone();
                            let name = name.clone();
                            async move {
                                rpc::handle_rpc(state, rls, schema, name, Json(body)).await
                            }
                        }),
                    )
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
/// window and total. RLS is enforced by `executor.execute_raw`, which
/// `acquire(rls)`s a connection and issues the `SET LOCAL` GUCs before this
/// statement runs — the caller's `RlsContext` is threaded from the bearer
/// token verified upstream by `require_rls` (`fdb-gateway`).
///
/// SECURITY: schema, table, and every filter column pass through
/// [`is_safe_identifier`]; all values are bound as `$n`. No user-supplied
/// string is interpolated into SQL.
#[instrument(skip(state, rls, params, headers), fields(schema = %schema, table = %table))]
async fn handle_list(
    state: RestState,
    rls: RlsContext,
    schema: String,
    table: String,
    params: HashMap<String, String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !is_safe_identifier(&schema) || !is_safe_identifier(&table) {
        return bad_request("invalid schema or table identifier");
    }

    // Resource embedding: parse the `select=` grammar (embeds + scalar columns) and
    // route embed-scoped params (`?child.col=…`, `order=child.col.dir`) onto the
    // matching embed. Non-embed params fall through to top-level filters.
    let embed_schema = embed_schema_from_model(&state.model);
    let inner = match build_inner_query(&schema, &table, &params, &embed_schema) {
        Ok(inner) => inner,
        Err(msg) => return bad_request(&msg),
    };

    let (offset, limit) = parse_range(&headers);
    // LIMIT/OFFSET placeholders follow every bound param already threaded.
    // No explicit cast: Postgres infers `int8` for a bare `LIMIT $n OFFSET
    // $n` parameter on its own. Bind as `QueryParam::BigInt`, not `Text` — an
    // explicit `::bigint` cast here would make Postgres infer the
    // placeholder's type as `int8`, and `Text`'s client-side `ToSql::accepts`
    // does not include `INT8`, which fails to serialize before the query
    // ever reaches the server (discovered running this change's own
    // live-Postgres gate test — see `QueryParam::BigInt`'s doc comment).
    let limit_idx = inner.binds.len() + 1;
    let offset_idx = inner.binds.len() + 2;

    let sql = format!(
        "SELECT COALESCE(json_agg(t), '[]'::json) AS rows, \
                (SELECT count(*) FROM {schema}.{table}) AS total \
         FROM ({inner_sql} \
               ORDER BY 1 LIMIT ${limit_idx} OFFSET ${offset_idx}) t",
        inner_sql = inner.sql,
    );

    let mut binds = inner.binds;
    binds.push(QueryParam::BigInt(limit));
    binds.push(QueryParam::BigInt(offset));

    match state.executor.execute_raw(&sql, binds, &rls).await {
        Ok(rows) => match rows.into_iter().next() {
            Some(row) => list_response(&row, offset, limit),
            None => list_response(&serde_json::Map::new(), offset, limit),
        },
        Err(e) => {
            tracing::error!(error = %e, "handle_list query error");
            internal_error()
        }
    }
}

/// The inner `SELECT … FROM … WHERE …` (before the json_agg wrapper / ORDER / LIMIT),
/// with all bound params threaded in `$1..$n` textual order.
struct InnerQuery {
    sql: String,
    binds: Vec<QueryParam>,
}

/// Build the inner list query, honoring PostgREST resource embedding.
///
/// Param order matches SQL text: projection embed params (SELECT clause) → WHERE
/// filter params → `!inner`/filter-by-embed EXISTS guard params. LIMIT/OFFSET are
/// appended by the caller after these.
///
/// When `select=` names no embeds, this is behaviorally identical to the previous
/// `SELECT * FROM <schema>.<table> <where>` (no alias, no subselects).
fn build_inner_query(
    schema: &str,
    table: &str,
    params: &HashMap<String, String>,
    embed_schema: &fdb_query::embed::EmbedSchema,
) -> Result<InnerQuery, String> {
    // Parse the select= grammar; route embed-scoped params onto their embeds.
    let mut embed_select = match params.get("select") {
        Some(raw) => parse_embed_select(raw).map_err(|e| e.to_string())?,
        None => parse_embed_select("").map_err(|e| e.to_string())?,
    };
    for (key, value) in params {
        if key == "select" {
            continue;
        }
        // Routed embed params are consumed here; the rest remain top-level filters.
        let _ = route_embedded_param(&mut embed_select, key, value).map_err(|e| e.to_string())?;
    }

    // Top-level filters: every non-reserved, non-embed-routed param.
    let filter_tree =
        parse_filter_tree_excluding_embeds(params, &embed_select).map_err(|e| e.to_string())?;

    // No embeds → the simple, previously-shipped shape (no parent alias).
    if embed_select.embeds.is_empty() {
        let where_clause = render_where(&filter_tree, 1)?;
        let sql = format!("SELECT * FROM {schema}.{table} {}", where_clause.sql);
        return Ok(InnerQuery {
            sql,
            binds: where_clause.binds,
        });
    }

    // Embedded path: alias the parent so correlation predicates can reference it.
    let parent_alias = table.to_owned();
    let resolved = resolve_embeds(&embed_select, table, &parent_alias, embed_schema)
        .map_err(|e| e.to_string())?;

    // 1) Projection: `<alias>.*` plus embed subselects. Params start at $1.
    let base = fdb_query::Select::default(); // renders "*"
    let (embed_items_sql, proj_params, after_proj) =
        render_projection(&base, &resolved, 1).map_err(|e| e.to_string())?;
    // render_projection prepends the base ("*"); qualify it to the parent alias.
    let projection = embed_items_sql.replacen('*', &format!("{parent_alias}.*"), 1);

    // 2) Top-level WHERE, params continue after the projection params.
    let where_clause = render_where(&filter_tree, after_proj)?;
    let after_where = after_proj + where_clause.binds.len();

    // 3) !inner / filter-by-embed EXISTS guards, params continue after WHERE.
    let (guard_preds, guard_params, _) =
        render_inner_guards(&resolved, after_where).map_err(|e| e.to_string())?;

    // Assemble WHERE: filter clause AND guards.
    let mut where_terms: Vec<String> = Vec::new();
    if !where_clause.sql.is_empty() {
        // where_clause.sql includes the leading "WHERE "; strip it for composition.
        where_terms.push(where_clause.sql.trim_start_matches("WHERE ").to_owned());
    }
    where_terms.extend(guard_preds);
    let where_sql = if where_terms.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_terms.join(" AND "))
    };

    let sql = format!("SELECT {projection} FROM {schema}.{table} {parent_alias} {where_sql}");

    let mut binds = proj_params;
    binds.extend(where_clause.binds);
    binds.extend(guard_params);
    Ok(InnerQuery { sql, binds })
}

/// Build the top-level filter tree from params, skipping reserved keys and any
/// param that was routed onto an embed (dotted `child.col` / `order=child…`).
fn parse_filter_tree_excluding_embeds(
    params: &HashMap<String, String>,
    embed_select: &fdb_query::embed::EmbedSelect,
) -> Result<fdb_query::FilterTree, crate::compilers::filters::FilterError> {
    // A param is embed-routed when its key's head segment names one of the embeds.
    let embed_names: std::collections::HashSet<&str> = embed_select
        .embeds
        .iter()
        .map(|e| e.alias.as_deref().unwrap_or(&e.target))
        .collect();
    let top: HashMap<String, String> = params
        .iter()
        .filter(|(k, _)| {
            if k.as_str() == "order" {
                return false; // order handled within the query builder / embeds
            }
            match k.split_once('.') {
                Some((head, _)) => !embed_names.contains(head),
                None => true,
            }
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    parse_filter_tree(&top)
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
fn list_response(
    row: &serde_json::Map<String, Value>,
    offset: i64,
    limit: i64,
) -> axum::response::Response {
    let rows: Value = row.get("rows").cloned().unwrap_or(Value::Array(vec![]));
    let total: i64 = row.get("total").and_then(Value::as_i64).unwrap_or(0);

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
pub(super) fn json_bind(v: &Value) -> QueryParam {
    match v {
        Value::String(s) => QueryParam::Text(s.clone()),
        other => QueryParam::Json(other.to_string()),
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

#[cfg(test)]
mod tests {
    use super::RestCompiler;
    use super::{build_inner_query, embed_schema_from_model};
    use crate::model::{Column, DatabaseModel, ForeignKey, Table};
    use fdb_query::QueryParam;
    use serde_json::Value;
    use std::collections::HashMap;

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

    fn col(name: &str) -> Column {
        Column {
            name: name.into(),
            pg_type: "text".into(),
            nullable: true,
            default: None,
        }
    }

    /// customers 1—* orders (orders.customer_id -> customers.id).
    fn embed_model() -> DatabaseModel {
        DatabaseModel {
            tables: vec![
                Table {
                    schema: "public".into(),
                    name: "customers".into(),
                    columns: vec![col("id"), col("name")],
                    pk: vec!["id".into()],
                    fk: vec![],
                    rls_enabled: true,
                    vault_key: None,
                },
                Table {
                    schema: "public".into(),
                    name: "orders".into(),
                    columns: vec![col("id"), col("customer_id"), col("total")],
                    pk: vec!["id".into()],
                    fk: vec![ForeignKey {
                        from_col: "customer_id".into(),
                        to_schema: "public".into(),
                        to_table: "customers".into(),
                        to_col: "id".into(),
                    }],
                    rls_enabled: true,
                    vault_key: None,
                },
            ],
            functions: vec![],
            views: vec![],
            version: 1,
        }
    }

    fn params(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn no_embed_path_is_unchanged_simple_select() {
        let m = embed_model();
        let es = embed_schema_from_model(&m);
        let inner = build_inner_query("public", "orders", &params(&[("total", "gte.100")]), &es)
            .expect("inner");
        assert_eq!(inner.sql, "SELECT * FROM public.orders WHERE total >= $1");
        assert_eq!(inner.binds.len(), 1);
    }

    #[test]
    fn embed_to_many_renders_correlated_subselect_aliased_parent() {
        let m = embed_model();
        let es = embed_schema_from_model(&m);
        // customers embedding their orders
        let inner = build_inner_query(
            "public",
            "customers",
            &params(&[("select", "*,orders(*)")]),
            &es,
        )
        .expect("inner");
        // Parent star qualified to the alias; embed rendered as a subselect; aliased FROM.
        assert!(inner.sql.contains("customers.*"), "sql: {}", inner.sql);
        assert!(
            inner.sql.contains("FROM public.customers customers"),
            "sql: {}",
            inner.sql
        );
        assert!(
            inner.sql.contains("json_agg"),
            "to-many embed uses json_agg: {}",
            inner.sql
        );
    }

    #[test]
    fn embed_scoped_filter_is_bound_and_routed() {
        let m = embed_model();
        let es = embed_schema_from_model(&m);
        // Filter the embedded orders by total; value must be bound, not interpolated.
        let inner = build_inner_query(
            "public",
            "customers",
            &params(&[("select", "*,orders(*)"), ("orders.total", "gt.50")]),
            &es,
        )
        .expect("inner");
        assert!(
            !inner.sql.contains("50"),
            "embed filter value must be bound: {}",
            inner.sql
        );
        assert!(!inner.binds.is_empty(), "embed filter contributes a bind");
    }

    #[test]
    fn unsafe_relation_in_embed_path_is_rejected() {
        let m = embed_model();
        let es = embed_schema_from_model(&m);
        // An unsafe parent table must be rejected by resolve_embeds validation.
        let r = build_inner_query(
            "public",
            "customers; DROP",
            &params(&[("select", "*,orders(*)")]),
            &es,
        );
        assert!(r.is_err(), "unsafe parent table must error");
    }

    /// Never-invoked `SqlExecutor` for tests that only exercise route
    /// registration (`RestCompiler::compile`), which never runs a query.
    struct UnusedExecutor;
    #[async_trait::async_trait]
    impl fdb_ports::SqlExecutor for UnusedExecutor {
        async fn execute_raw(
            &self,
            _sql: &str,
            _params: Vec<QueryParam>,
            _rls: &forge_identity::RlsContext,
        ) -> Result<Vec<serde_json::Map<String, Value>>, fdb_ports::BackendError> {
            unreachable!("compile() never executes a query")
        }
    }

    #[tokio::test]
    async fn compiles_without_panic_for_minimal_model() {
        // compile() must not panic during route registration; it never runs a query.
        let model = minimal_model();
        let _router = RestCompiler::compile(&model, std::sync::Arc::new(UnusedExecutor));
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
