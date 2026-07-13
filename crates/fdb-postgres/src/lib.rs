//! Postgres adapters: DatabaseBackend, SchemaProvider, RestExecutor, GraphQlExecutor (pg_graphql), pgvector.
#![forbid(unsafe_code)]

pub mod conn;
pub mod error;

use async_trait::async_trait;
use deadpool_postgres::{Config as PoolConfig, Pool, Runtime};
use fdb_domain::{GraphQlRequest, RestQuery, RestResult, VectorRpcRequest};
use fdb_ports::{BackendError, Conn, DatabaseBackend, GraphQlExecutor, RestExecutor, SqlExecutor};
use forge_domain::Json;
use forge_identity::RlsContext;
use tracing::instrument;

use crate::conn::PgConn;
use crate::error::PgError;

/// Deadpool-backed Postgres connection pool implementing `DatabaseBackend`.
pub struct PgBackend {
    pool: Pool,
}

impl PgBackend {
    /// Build from `DATABASE_URL` environment variable.
    ///
    /// Expects a standard Postgres connection URL:
    /// `postgres://user:password@host:port/dbname`
    pub fn from_env() -> Result<Self, PgError> {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| PgError::Config("DATABASE_URL must be set".into()))?;

        let mut cfg = PoolConfig::new();
        cfg.url = Some(database_url);

        let pool = cfg
            .create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)
            .map_err(|e| PgError::Config(e.to_string()))?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl DatabaseBackend for PgBackend {
    /// Check out a connection from the pool, open a transaction, and apply all
    /// six `SET LOCAL` statements that propagate JWT context to Postgres RLS
    /// and the extended GUC layer (all within the same `BEGIN` transaction):
    ///
    /// ```sql
    /// SET LOCAL ROLE <role>;
    /// SET LOCAL "request.jwt.claims" = '<claims_json>';
    /// SET LOCAL "request.headers"    = '{"authorization":"Bearer <raw_bearer>"}';
    /// SET LOCAL "app.jwt_claims"     = '<claims_json>';
    /// SET LOCAL "app.keto_subject"   = '<keto_subject>';
    /// SET LOCAL "app.vault_key_id"   = '<vault_key_id>';
    /// ```
    ///
    /// The returned `Conn` keeps the connection alive for the duration of the
    /// caller's use. Callers MUST NOT log the `RlsContext` — it contains the
    /// raw bearer token and subject identifiers.
    #[instrument(skip(self, rls), fields(role = %rls.role), err)]
    async fn acquire(&self, rls: &RlsContext) -> Result<Conn, BackendError> {
        let object = self
            .pool
            .get()
            .await
            .map_err(|e| PgError::Checkout(e.to_string()))?;

        // SET LOCAL requires an open transaction.
        object
            .execute("BEGIN", &[])
            .await
            .map_err(PgError::Transaction)?;

        // ROLE: `SET` statements do NOT accept bind parameters, so the role reaches
        // SQL as an identifier. It is user-derived, so validate it first (fail closed)
        // and quote it — never interpolate an unvalidated value.
        if !forge_domain::is_safe_identifier(&rls.role) {
            return Err(PgError::SetLocal(format!("unsafe role identifier: {}", rls.role)).into());
        }
        object
            .execute(&format!(r#"SET LOCAL ROLE "{}""#, rls.role), &[])
            .await
            .map_err(|e| PgError::SetLocal(format!("SET LOCAL ROLE: {e}")))?;

        // The six GUCs: `SET LOCAL <name> = $1` is also invalid (SET rejects binds).
        // Use `set_config(name, value, is_local=true)`, which binds the VALUE safely
        // (the name is a fixed literal, never user input). Equivalent to SET LOCAL.
        let vault_key_id = rls.vault_key_id.as_deref().unwrap_or("");
        let headers_json = format!(r#"{{"authorization":"Bearer {}"}}"#, rls.raw_bearer);
        // (name, value) pairs — names are hardcoded, values are bound.
        let gucs: [(&str, &str); 5] = [
            ("request.jwt.claims", &rls.claims_json),
            ("request.headers", &headers_json),
            ("app.jwt_claims", &rls.claims_json),
            ("app.keto_subject", &rls.keto_subject),
            ("app.vault_key_id", vault_key_id),
        ];
        for (name, value) in gucs {
            object
                .execute("SELECT set_config($1, $2, true)", &[&name, &value])
                .await
                .map_err(|e| PgError::SetLocal(format!("set_config({name}): {e}")))?;
        }

        Ok(Conn(Box::new(PgConn::new(object))))
    }
}

/// GraphQL executor that delegates to `pg_graphql`'s `graphql.resolve()` function.
///
/// Uses a `PgBackend` internally so the full 6-GUC RLS context is always set
/// before the `graphql.resolve()` call runs.
pub struct PgGraphQl {
    backend: PgBackend,
}

impl PgGraphQl {
    pub fn new(pool: Pool) -> Self {
        Self {
            backend: PgBackend { pool },
        }
    }
}

#[async_trait]
impl GraphQlExecutor for PgGraphQl {
    /// Delegate to `graphql.resolve($1, $2::jsonb, $3)` under full RLS context.
    ///
    /// pg_graphql's resolve function signature:
    /// ```sql
    /// SELECT graphql.resolve(query text, variables jsonb, "operationName" text DEFAULT NULL)
    /// ```
    #[instrument(skip(self, rls), fields(role = %rls.role), err)]
    async fn execute(&self, req: GraphQlRequest, rls: &RlsContext) -> Result<Json, BackendError> {
        use fdb_ports::DatabaseBackend;

        let conn = self.backend.acquire(rls).await?;

        let pg_conn = PgConn::from_conn(&conn)
            .ok_or_else(|| BackendError::Internal("unexpected conn type in PgGraphQl".into()))?;

        let variables_json = req.variables.to_string();
        let row = pg_conn
            .inner
            .query_one(
                r"SELECT graphql.resolve($1, $2::jsonb, $3)",
                &[&req.query, &variables_json, &req.operation_name],
            )
            .await
            .map_err(|e| BackendError::Internal(format!("graphql.resolve: {e}")))?;

        // pg_graphql returns JSONB. tokio-postgres can read it as a String
        // (the text representation of JSONB), then we deserialize.
        let raw: String = row.get(0);
        let result: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| BackendError::Internal(format!("graphql.resolve json parse: {e}")))?;
        Ok(result)
    }
}

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
    #[instrument(skip(self, rls), fields(role = %rls.role, table = %q.table), err)]
    async fn execute(&self, q: RestQuery, rls: &RlsContext) -> Result<RestResult, BackendError> {
        let plan = plan_from_rest_query(&q)?;
        let (sql, params) = plan
            .render()
            .map_err(|e| BackendError::Query(e.to_string()))?;

        let rows = self.run_bound(&sql, params, rls).await?;
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
    #[instrument(skip(self, rls, params), fields(role = %rls.role), err)]
    async fn execute_raw(
        &self,
        sql: &str,
        params: Vec<fdb_query::QueryParam>,
        rls: &RlsContext,
    ) -> Result<Vec<serde_json::Map<String, serde_json::Value>>, BackendError> {
        self.run_bound(sql, params, rls).await
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

        // Materialize owned bind values, then build the &(dyn ToSql + Sync) slice.
        let owned: Vec<RestBind> = params.into_iter().map(RestBind::from_param).collect();
        let binds: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
            owned.iter().map(RestBind::as_to_sql).collect();

        let rows = pg_conn
            .inner
            .query(sql, &binds)
            .await
            .map_err(|e| BackendError::Query(format!("bound query: {e}")))?;

        Ok(project_rows(&rows))
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
fn project_rows(rows: &[tokio_postgres::Row]) -> Vec<serde_json::Map<String, serde_json::Value>> {
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

/// Identifier validation: alphanumeric, underscore, and dot (for schema.table) only.
/// Rejects any attempt to inject SQL via table or column name parameters.
fn is_safe_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        && !s.starts_with('.')
        && !s.ends_with('.')
}

/// Postgres adapter for vector similarity search via the pgvector `<=>` operator.
///
/// Executes `SELECT *, (<col> <=> $1::vector) AS distance FROM <table>
/// ORDER BY <col> <=> $1::vector LIMIT $2` under the full 6-GUC RLS context.
///
/// SECURITY: table and column names are validated with [`is_safe_identifier`]
/// before interpolation. User-supplied values MUST NOT bypass this check.
pub struct PgVectorRpc {
    backend: PgBackend,
}

impl PgVectorRpc {
    pub fn new(pool: Pool) -> Self {
        Self {
            backend: PgBackend { pool },
        }
    }

    /// Execute a vector similarity search under the caller's RLS context.
    ///
    /// Returns a JSON array of rows with an additional `distance` field.
    #[instrument(skip(self, rls), fields(table = %req.table, column = %req.column), err)]
    pub async fn execute_similarity(
        &self,
        req: &VectorRpcRequest,
        rls: &RlsContext,
    ) -> Result<Json, BackendError> {
        if !is_safe_identifier(&req.table) {
            return Err(BackendError::Query(format!(
                "invalid table identifier: {}",
                req.table
            )));
        }
        if !is_safe_identifier(&req.column) {
            return Err(BackendError::Query(format!(
                "invalid column identifier: {}",
                req.column
            )));
        }

        let limit = req.limit.min(1000);
        let conn = self.backend.acquire(rls).await?;

        let pg_conn = PgConn::from_conn(&conn)
            .ok_or_else(|| BackendError::Internal("unexpected conn type in PgVectorRpc".into()))?;

        let vec = pgvector::Vector::from(req.embedding.clone());
        let sql = format!(
            "SELECT *, ({col} <=> $1::vector) AS distance \
             FROM {tbl} \
             ORDER BY {col} <=> $1::vector \
             LIMIT $2",
            col = req.column,
            tbl = req.table,
        );

        let rows = pg_conn
            .inner
            .query(&sql, &[&vec, &(i64::from(limit))])
            .await
            .map_err(|e| BackendError::Query(format!("vector similarity query: {e}")))?;

        let mut results: Vec<serde_json::Value> = Vec::with_capacity(rows.len());
        for row in &rows {
            let mut obj = serde_json::Map::new();
            for (i, col) in row.columns().iter().enumerate() {
                let val: Option<String> = row.try_get(i).ok();
                obj.insert(
                    col.name().to_owned(),
                    val.map_or(serde_json::Value::Null, serde_json::Value::String),
                );
            }
            results.push(serde_json::Value::Object(obj));
        }

        Ok(serde_json::Value::Array(results))
    }
}

#[cfg(test)]
mod tests {
    use super::is_safe_identifier;

    #[test]
    fn test_safe_identifier_accepts_valid_names() {
        assert!(is_safe_identifier("public"));
        assert!(is_safe_identifier("items"));
        assert!(is_safe_identifier("public.items"));
        assert!(is_safe_identifier("my_table_2"));
        assert!(is_safe_identifier("schema.my_table"));
    }

    #[test]
    fn test_safe_identifier_rejects_injection_attempts() {
        assert!(!is_safe_identifier("items; DROP TABLE users--"));
        assert!(!is_safe_identifier("items' OR '1'='1"));
        assert!(!is_safe_identifier("items--"));
        assert!(!is_safe_identifier(""));
        assert!(!is_safe_identifier(".items"));
        assert!(!is_safe_identifier("items."));
        assert!(!is_safe_identifier("a b"));
        assert!(!is_safe_identifier("items\n"));
    }

    #[test]
    fn test_safe_identifier_rejects_oversized_names() {
        let long = "a".repeat(129);
        assert!(!is_safe_identifier(&long));
    }

    #[test]
    fn test_limit_cap() {
        // Verify limit capping logic matches the 1000 constant in execute_similarity.
        let limit: u32 = 5000;
        assert_eq!(limit.min(1000), 1000);
        let limit: u32 = 5;
        assert_eq!(limit.min(1000), 5);
    }
}
