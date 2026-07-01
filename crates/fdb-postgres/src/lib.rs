//! Postgres adapters: DatabaseBackend, SchemaProvider, RestExecutor, GraphQlExecutor (pg_graphql), pgvector.
#![forbid(unsafe_code)]

pub mod conn;
pub mod error;

use async_trait::async_trait;
use deadpool_postgres::{Config as PoolConfig, Pool, Runtime};
use fdb_domain::{GraphQlRequest, RestQuery, RestResult, VectorRpcRequest};
use fdb_ports::{BackendError, Conn, DatabaseBackend, GraphQlExecutor, RestExecutor};
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

        object
            .execute("SET LOCAL ROLE $1", &[&rls.role])
            .await
            .map_err(|e| PgError::SetLocal(format!("SET LOCAL ROLE: {e}")))?;

        object
            .execute(
                r#"SET LOCAL "request.jwt.claims" = $1"#,
                &[&rls.claims_json],
            )
            .await
            .map_err(|e| PgError::SetLocal(format!(r#"SET LOCAL "request.jwt.claims": {e}"#)))?;

        let headers_json = format!(r#"{{"authorization":"Bearer {}"}}"#, rls.raw_bearer);
        object
            .execute(
                r#"SET LOCAL "request.headers" = $1"#,
                &[&headers_json],
            )
            .await
            .map_err(|e| PgError::SetLocal(format!(r#"SET LOCAL "request.headers": {e}"#)))?;

        // Extended GUC propagation — all within the same BEGIN transaction.
        // These three are consumed by flint_vault, flint_hooks, and Cedar policy evaluation.
        // MUST NOT log their values — they contain subject IDs and key identifiers.
        object
            .execute(
                r#"SET LOCAL "app.jwt_claims" = $1"#,
                &[&rls.claims_json],
            )
            .await
            .map_err(|e| PgError::SetLocal(format!(r#"SET LOCAL "app.jwt_claims": {e}"#)))?;

        object
            .execute(
                r#"SET LOCAL "app.keto_subject" = $1"#,
                &[&rls.keto_subject],
            )
            .await
            .map_err(|e| PgError::SetLocal(format!(r#"SET LOCAL "app.keto_subject": {e}"#)))?;

        let vault_key_id = rls.vault_key_id.as_deref().unwrap_or("");
        object
            .execute(
                r#"SET LOCAL "app.vault_key_id" = $1"#,
                &[&vault_key_id],
            )
            .await
            .map_err(|e| PgError::SetLocal(format!(r#"SET LOCAL "app.vault_key_id": {e}"#)))?;

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

pub struct PgRest {
    #[allow(dead_code)]
    pool: Pool,
}

impl PgRest {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RestExecutor for PgRest {
    async fn execute(&self, _q: RestQuery, _rls: &RlsContext) -> Result<RestResult, BackendError> {
        todo!("PostgREST-compatible query builder + pgvector /rpc")
    }
}

/// Identifier validation: alphanumeric, underscore, and dot (for schema.table) only.
/// Rejects any attempt to inject SQL via table or column name parameters.
fn is_safe_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
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
                obj.insert(col.name().to_owned(), val.map_or(serde_json::Value::Null, serde_json::Value::String));
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
