//! `PgGraphQl`: `GraphQlExecutor` adapter delegating to `pg_graphql`.

use async_trait::async_trait;
use deadpool_postgres::Pool;
use fdb_domain::GraphQlRequest;
use fdb_ports::{BackendError, GraphQlExecutor};
use forge_domain::Json;
use forge_identity::RlsContext;
use tracing::instrument;

use crate::conn::PgConn;
use crate::PgBackend;

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
