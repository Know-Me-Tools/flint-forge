//! `PgRest`: PostgREST-compatible `RestExecutor`/`SqlExecutor` adapter.

use async_trait::async_trait;
use deadpool_postgres::Pool;
use fdb_domain::{RestQuery, RestResult};
use fdb_ports::{BackendError, IdempotencyContext, RestExecutor, SqlExecutor};
use forge_identity::RlsContext;
use tracing::instrument;

use crate::conn::PgConn;
use crate::PgBackend;

/// PostgREST-compatible REST executor.
///
/// Translates a [`RestQuery`] into SQL via the pure [`fdb_query`] planner, then
/// runs it under the full 6-GUC RLS context (`backend.acquire(rls)`). The planner
/// guarantees identifier validation and parameter binding; this adapter only binds
/// values and projects rows.
pub struct PgRest {
    backend: PgBackend,
}

impl PgRest {
    /// Wrap an existing deadpool `Pool` in a `PgRest` executor.
    ///
    /// The pool is shared, not owned exclusively — the same `Pool` is
    /// typically also handed to `PgGraphQl`/`PgVectorRpc` so all executors
    /// draw from one connection budget.
    #[must_use]
    pub fn new(pool: Pool) -> Self {
        Self {
            backend: PgBackend { pool },
        }
    }
}

/// Convert a `RestQuery` (already-parsed filter tuples) into an `fdb_query`
/// `FilterTree` — an AND of leaves. The op token is validated by the planner.
fn rest_query_to_filter(q: &RestQuery) -> Result<fdb_query::FilterTree, BackendError> {
    let mut leaves = Vec::with_capacity(q.filters.len());
    for (col, op_token, value) in &q.filters {
        let op = fdb_query::Operator::parse(op_token)
            .ok_or_else(|| BackendError::Query(format!("unknown operator: {op_token}")))?;
        leaves.push(fdb_query::FilterTree::Leaf {
            column: col.clone(),
            op,
            value: value.clone(),
            negate: false,
            quantifier: None,
            fts_config: None,
        });
    }
    Ok(fdb_query::FilterTree::And(leaves))
}

/// Build the `fdb_query::SelectPlan` for a `RestQuery`.
fn plan_from_rest_query(q: &RestQuery) -> Result<fdb_query::SelectPlan, BackendError> {
    let relation = if q.schema.is_empty() {
        q.table.clone()
    } else {
        format!("{}.{}", q.schema, q.table)
    };
    let select = match &q.select {
        Some(s) => fdb_query::Select::parse(s).map_err(|e| BackendError::Query(e.to_string()))?,
        None => fdb_query::Select::default(),
    };
    let order = match &q.order {
        Some(o) => fdb_query::Order::parse(o).map_err(|e| BackendError::Query(e.to_string()))?,
        None => fdb_query::Order::default(),
    };
    Ok(fdb_query::SelectPlan {
        relation: fdb_query::validate_identifier(&relation)
            .map_err(|_| BackendError::Query(format!("unsafe relation: {relation}")))?
            .to_owned(),
        select,
        filter: rest_query_to_filter(q)?,
        order,
        limits: fdb_query::Limits::from_params(q.limit.map(u64::from), q.offset.map(u64::from)),
        count: fdb_query::CountStrategy::None,
    })
}

#[async_trait]
impl RestExecutor for PgRest {
    /// Execute a REST list/read query under RLS.
    ///
    /// The plan is rendered by `fdb_query` (validated identifiers, bound params);
    /// this method binds the parameters and projects rows to a JSON array.
    ///
    /// # Errors
    ///
    /// Returns `BackendError` when: `q.filters` references an unknown filter
    /// operator token; `q.select`/`q.order` fail to parse; the target
    /// `schema.table` relation fails identifier validation (rejecting
    /// injection attempts); or the underlying `run_bound` call fails
    /// (RLS-context acquisition or the bound query itself).
    #[instrument(skip(self, rls), fields(role = %rls.role, table = %q.table), err)]
    async fn execute(&self, q: RestQuery, rls: &RlsContext) -> Result<RestResult, BackendError> {
        let plan = plan_from_rest_query(&q)?;
        let (sql, params) = plan
            .render()
            .map_err(|e| BackendError::Query(e.to_string()))?;
        // Serialize the selected row in Postgres so native values such as UUID,
        // numeric, date/time and arrays keep their JSON representation. Reading
        // arbitrary projected columns through `tokio_postgres::Row` would require
        // an exhaustive Rust decoder and previously turned UUID tenant keys into
        // `null` on the subscription RLS re-query path.
        let row_sql = format!("SELECT row_to_json(_forge_row) AS row FROM ({sql}) AS _forge_row");
        let projected = self.run_bound(&row_sql, params, rls).await?;
        let mut rows = Vec::with_capacity(projected.len());
        for mut projected_row in projected {
            match projected_row.remove("row") {
                Some(serde_json::Value::Object(row)) => rows.push(row),
                _ => {
                    return Err(BackendError::Internal(
                        "Postgres row projection did not return a JSON object".into(),
                    ));
                }
            }
        }
        let count = Some(rows.len() as u64);
        Ok(RestResult {
            rows: serde_json::Value::Array(
                rows.into_iter().map(serde_json::Value::Object).collect(),
            ),
            count,
        })
    }
}

#[async_trait]
impl SqlExecutor for PgRest {
    /// Run an already-rendered `(sql, params)` pair — produced by `fdb-reflection`
    /// via the same `fdb_query` planner `RestExecutor::execute` uses above — inside
    /// an RLS-scoped transaction. This is the seam `fdb-reflection`'s REST CRUD and
    /// `/rpc` handlers use so they never touch a raw, unscoped connection.
    ///
    /// # Errors
    ///
    /// Returns `BackendError` when acquiring the RLS-scoped connection fails,
    /// the checked-out connection is not a `PgConn`, or `sql` fails to
    /// execute (syntax error, constraint violation, RLS denial, etc.).
    #[instrument(skip(self, rls, params), fields(role = %rls.role))]
    async fn execute_raw(
        &self,
        sql: &str,
        params: Vec<fdb_query::QueryParam>,
        rls: &RlsContext,
    ) -> Result<Vec<serde_json::Map<String, serde_json::Value>>, BackendError> {
        self.run_bound(sql, params, rls).await
    }

    #[instrument(skip(self, rls, params, idempotency), fields(role = %rls.role))]
    async fn execute_idempotent(
        &self,
        sql: &str,
        params: Vec<fdb_query::QueryParam>,
        rls: &RlsContext,
        idempotency: &IdempotencyContext,
    ) -> Result<Vec<serde_json::Map<String, serde_json::Value>>, BackendError> {
        use fdb_ports::DatabaseBackend;

        let subject = rls.subject().ok_or(BackendError::Denied)?.0;
        let conn = self.backend.acquire(rls).await?;
        let pg_conn = PgConn::from_conn(&conn)
            .ok_or_else(|| BackendError::Internal("unexpected conn type in PgRest".into()))?;

        let lock_key = format!("{subject}:{}", idempotency.mutation_id);
        if let Err(error) = pg_conn
            .inner
            .query_one(
                "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
                &[&lock_key],
            )
            .await
        {
            rollback(pg_conn).await?;
            return Err(map_query_error("idempotency lock", error));
        }

        let existing = match pg_conn
            .inner
            .query_opt(
                "SELECT request_fingerprint, response FROM flint.mutation_receipts WHERE subject = $1 AND mutation_id = $2",
                &[&subject, &idempotency.mutation_id],
            )
            .await
        {
            Ok(row) => row,
            Err(error) => {
                rollback(pg_conn).await?;
                return Err(map_query_error("idempotency receipt lookup", error));
            }
        };

        if let Some(row) = existing {
            let fingerprint: String = row.get(0);
            if fingerprint != idempotency.request_fingerprint {
                rollback(pg_conn).await?;
                return Err(BackendError::IdempotencyConflict);
            }
            let response: serde_json::Value = row.get(1);
            let rows = serde_json::from_value(response).map_err(|error| {
                BackendError::Internal(format!("invalid mutation receipt: {error}"))
            })?;
            pg_conn.commit().await?;
            return Ok(rows);
        }

        let rows = match query_bound(pg_conn, sql, params).await {
            Ok(rows) => rows,
            Err(error) => {
                rollback(pg_conn).await?;
                return Err(error);
            }
        };
        let response = serde_json::to_value(&rows).map_err(|error| {
            BackendError::Internal(format!("serialize mutation receipt: {error}"))
        })?;
        if let Err(error) = pg_conn
            .inner
            .execute(
                "INSERT INTO flint.mutation_receipts (subject, mutation_id, request_fingerprint, response) VALUES ($1, $2, $3, $4)",
                &[&subject, &idempotency.mutation_id, &idempotency.request_fingerprint, &response],
            )
            .await
        {
            rollback(pg_conn).await?;
            return Err(map_query_error("idempotency receipt insert", error));
        }
        pg_conn.commit().await?;
        Ok(rows)
    }
}

impl PgRest {
    /// Shared bind + execute + row-projection: `acquire(rls)` the connection,
    /// bind `params` in `$n` order, run `sql`, project each row to a JSON object
    /// keyed by column name. Used by both [`RestExecutor::execute`] (which
    /// builds `sql`/`params` from a `RestQuery`) and [`SqlExecutor::execute_raw`]
    /// (which takes an already-rendered `sql`/`params` pair directly).
    async fn run_bound(
        &self,
        sql: &str,
        params: Vec<fdb_query::QueryParam>,
        rls: &RlsContext,
    ) -> Result<Vec<serde_json::Map<String, serde_json::Value>>, BackendError> {
        use fdb_ports::DatabaseBackend;

        let conn = self.backend.acquire(rls).await?;
        let pg_conn = PgConn::from_conn(&conn)
            .ok_or_else(|| BackendError::Internal("unexpected conn type in PgRest".into()))?;

        let rows = match query_bound(pg_conn, sql, params).await {
            Ok(rows) => rows,
            Err(error) => {
                rollback(pg_conn).await?;
                return Err(error);
            }
        };

        // Commit the RLS transaction `acquire` opened. Without this every
        // INSERT/UPDATE/DELETE routed through here is rolled back when the
        // pooled connection is recycled, while still reporting success — the
        // `RETURNING` rows above are read inside the same doomed transaction.
        // See `PgConn::commit`.
        pg_conn.commit().await?;

        Ok(rows)
    }
}

async fn query_bound(
    pg_conn: &PgConn,
    sql: &str,
    params: Vec<fdb_query::QueryParam>,
) -> Result<Vec<serde_json::Map<String, serde_json::Value>>, BackendError> {
    let owned: Vec<RestBind> = params.into_iter().map(RestBind::from_param).collect();
    let binds: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
        owned.iter().map(RestBind::as_to_sql).collect();
    pg_conn
        .inner
        .query(sql, &binds)
        .await
        .map(|rows| project_rows(&rows))
        .map_err(|error| map_query_error("bound query", error))
}

async fn rollback(pg_conn: &PgConn) -> Result<(), BackendError> {
    pg_conn
        .inner
        .batch_execute("ROLLBACK")
        .await
        .map_err(|_| BackendError::Connection)
}

fn map_query_error(context: &str, error: tokio_postgres::Error) -> BackendError {
    if error.code() == Some(&tokio_postgres::error::SqlState::INSUFFICIENT_PRIVILEGE) {
        BackendError::Denied
    } else if error.code() == Some(&tokio_postgres::error::SqlState::UNIQUE_VIOLATION) {
        BackendError::ConstraintConflict
    } else {
        BackendError::Query(format!("{context}: {error}"))
    }
}

/// Project `tokio_postgres::Row`s to JSON objects keyed by column name.
///
/// Every `fdb-reflection` handler wraps its meaningful output as a single
/// `json`/`jsonb` column (`row_to_json`/`json_agg`), with `handle_list` adding
/// a `bigint` count sidecar — so this dispatches on the column's Postgres
/// `Type` rather than blindly decoding as text. `String`'s `FromSql::accepts`
/// only matches `TEXT`/`VARCHAR`/`BPCHAR`/`NAME`/`UNKNOWN`
/// (`postgres-types` `accepts!` list) — it does **not** accept `JSON`/`JSONB`/
/// integer/boolean/float types, so a naive `Option<String>` decode would
/// silently return `null` for exactly the columns this port exists to carry.
/// `serde_json::Value: FromSql` is available via the workspace's
/// `tokio-postgres = { features = ["with-serde_json-1"] }`.
///
/// Falls back to a text decode, then `NULL`, for any column type not listed
/// below — a reasonable degradation for a general-purpose executor, matching
/// the existing fallback convention in [`RestBind::from_param`].
///
/// `pub(crate)`: also reused by [`crate::vector_rpc`] for the same reason —
/// `pgvector` similarity results include non-text columns (notably the
/// `distance` float itself), so a single-type decode there would go stale.
pub(crate) fn project_rows(
    rows: &[tokio_postgres::Row],
) -> Vec<serde_json::Map<String, serde_json::Value>> {
    use tokio_postgres::types::Type;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let mut obj = serde_json::Map::new();
        for (i, col) in row.columns().iter().enumerate() {
            let value = match *col.type_() {
                Type::JSON | Type::JSONB => row
                    .try_get::<_, Option<serde_json::Value>>(i)
                    .ok()
                    .flatten()
                    .unwrap_or(serde_json::Value::Null),
                Type::BOOL => row
                    .try_get::<_, Option<bool>>(i)
                    .ok()
                    .flatten()
                    .map_or(serde_json::Value::Null, serde_json::Value::from),
                Type::INT2 => row
                    .try_get::<_, Option<i16>>(i)
                    .ok()
                    .flatten()
                    .map_or(serde_json::Value::Null, serde_json::Value::from),
                Type::INT4 => row
                    .try_get::<_, Option<i32>>(i)
                    .ok()
                    .flatten()
                    .map_or(serde_json::Value::Null, serde_json::Value::from),
                Type::INT8 => row
                    .try_get::<_, Option<i64>>(i)
                    .ok()
                    .flatten()
                    .map_or(serde_json::Value::Null, serde_json::Value::from),
                Type::FLOAT4 => row
                    .try_get::<_, Option<f32>>(i)
                    .ok()
                    .flatten()
                    .map_or(serde_json::Value::Null, |f| {
                        serde_json::Value::from(f64::from(f))
                    }),
                Type::FLOAT8 => row
                    .try_get::<_, Option<f64>>(i)
                    .ok()
                    .flatten()
                    .map_or(serde_json::Value::Null, serde_json::Value::from),
                _ => row
                    .try_get::<_, Option<String>>(i)
                    .ok()
                    .flatten()
                    .map_or(serde_json::Value::Null, serde_json::Value::String),
            };
            obj.insert(col.name().to_owned(), value);
        }
        out.push(obj);
    }
    out
}

/// Owned bind value bridging `fdb_query::QueryParam` to a `tokio_postgres` param.
enum RestBind {
    Text(String),
    TextArray(Vec<String>),
    Json(serde_json::Value),
    Vector(pgvector::Vector),
    BigInt(i64),
    Null,
}

impl RestBind {
    fn from_param(p: fdb_query::QueryParam) -> Self {
        match p {
            fdb_query::QueryParam::Text(s) => Self::Text(s),
            fdb_query::QueryParam::TextArray(v) => Self::TextArray(v),
            fdb_query::QueryParam::Json(s) => {
                Self::Json(serde_json::from_str(&s).unwrap_or(serde_json::Value::String(s)))
            }
            fdb_query::QueryParam::Vector(v) => Self::Vector(pgvector::Vector::from(v)),
            fdb_query::QueryParam::BigInt(n) => Self::BigInt(n),
            // `Null` and any future (`#[non_exhaustive]`) variant bind as NULL,
            // never panicking on a live query path.
            _ => Self::Null,
        }
    }

    fn as_to_sql(&self) -> &(dyn tokio_postgres::types::ToSql + Sync) {
        match self {
            Self::Text(s) => s,
            Self::TextArray(v) => v,
            Self::Json(j) => j,
            Self::BigInt(n) => n,
            Self::Vector(v) => v,
            Self::Null => &Option::<String>::None,
        }
    }
}
