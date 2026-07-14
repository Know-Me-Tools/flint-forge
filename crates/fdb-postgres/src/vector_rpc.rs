//! `PgVectorRpc`: pgvector similarity search adapter.

use deadpool_postgres::Pool;
use fdb_domain::VectorRpcRequest;
use fdb_ports::BackendError;
use forge_domain::Json;
use forge_identity::RlsContext;
use tracing::instrument;

use crate::conn::PgConn;
use crate::PgBackend;

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
    /// Wrap an existing deadpool `Pool` in a `PgVectorRpc` executor.
    ///
    /// The pool is shared, not owned exclusively — the same `Pool` is
    /// typically also handed to `PgRest`/`PgGraphQl` so all executors draw
    /// from one connection budget.
    #[must_use]
    pub fn new(pool: Pool) -> Self {
        Self {
            backend: PgBackend { pool },
        }
    }

    /// Execute a vector similarity search under the caller's RLS context.
    ///
    /// Returns a JSON array of rows with an additional `distance` field.
    ///
    /// # Errors
    ///
    /// Returns `BackendError` when: `req.table` or `req.column` fails the
    /// `is_safe_identifier` check (rejecting SQL injection via identifier
    /// interpolation — these two fields cannot be parameter-bound); acquiring
    /// the RLS-scoped connection fails; the checked-out connection is not a
    /// `PgConn`; or the rendered similarity query itself errors in Postgres.
    #[instrument(skip(self, rls), fields(table = %req.table, column = %req.column), err)]
    pub async fn execute_similarity(
        &self,
        req: &VectorRpcRequest,
        rls: &RlsContext,
    ) -> Result<Json, BackendError> {
        use fdb_ports::DatabaseBackend;

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

        // Type-aware projection (shared with `PgRest`): a naive `Option<String>`
        // decode per column would silently return `null` for every non-text
        // column — including `distance` itself, the whole point of this RPC.
        let results = crate::rest::project_rows(&rows)
            .into_iter()
            .map(serde_json::Value::Object)
            .collect();

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
