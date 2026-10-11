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
    /// Wrap an existing deadpool `Pool` in a `PgGraphQl` executor.
    ///
    /// The pool is shared with (not owned exclusively by) this adapter — the
    /// same `Pool` is typically also handed to `PgRest`/`PgVectorRpc` so all
    /// executors draw from one connection budget.
    #[must_use]
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
    ///
    /// # Errors
    ///
    /// Returns `BackendError` when: acquiring the RLS-scoped connection fails
    /// (pool checkout, the `BEGIN`, or any of the six `SET LOCAL`/`set_config`
    /// statements); the checked-out connection is not a `PgConn` (should not
    /// happen — indicates a mismatched `DatabaseBackend` implementation); or
    /// `graphql.resolve(...)` itself errors in Postgres (invalid query,
    /// permission denied under RLS, etc.).
    ///
    /// # Panics
    ///
    /// Does not panic under normal operation. It DID before this fix: decoding
    /// the `jsonb` result column via `row.get::<_, String>(0)` panics rather
    /// than erroring on a type mismatch, and `jsonb` is not in `FromSql for
    /// String`'s accepted OID set. Decoding as `serde_json::Value` directly
    /// removes the mismatch.
    #[instrument(skip(self, rls), fields(role = %rls.role), err)]
    async fn execute(&self, req: GraphQlRequest, rls: &RlsContext) -> Result<Json, BackendError> {
        use fdb_ports::DatabaseBackend;

        let conn = self.backend.acquire(rls).await?;

        let pg_conn = PgConn::from_conn(&conn)
            .ok_or_else(|| BackendError::Internal("unexpected conn type in PgGraphQl".into()))?;

        // Bound as `&serde_json::Value`, NOT `&req.variables.to_string()` (a `String`). The `::jsonb`
        // cast right after `$2` makes Postgres describe that parameter's type as `jsonb` (verified:
        // `parameter_types` on the prepared statement reads `{text,jsonb,text}`), and
        // `postgres-types`'s `ToSql for String` only accepts `TEXT`/`VARCHAR`/`BPCHAR`/`NAME` — not
        // `JSONB` — so the client rejects the bind before the query ever reaches the server, with
        // `error serializing parameter 1`, no matter what the JSON content is. Every request through
        // this path failed 100% of the time until this fix, unconditionally — the earlier `String`
        // round-trip through `.to_string()` was itself the bug, not an equivalent alternate encoding.
        // `serde_json::Value`'s `ToSql` impl (from `tokio-postgres`'s `with-serde_json-1` feature,
        // already enabled workspace-wide) accepts `JSON`/`JSONB` directly.
        let row = pg_conn
            .inner
            .query_one(
                r"SELECT graphql.resolve($1, $2::jsonb, $3)",
                &[&req.query, &req.variables, &req.operation_name],
            )
            .await
            .map_err(|e| BackendError::Internal(format!("graphql.resolve: {e}")))?;

        // `graphql.resolve` serves Mutation as well as Query (§3.2), so this
        // path writes and must commit the RLS transaction — otherwise every
        // GraphQL mutation is rolled back on connection recycle while still
        // returning a success payload. See `PgConn::commit`.
        pg_conn.commit().await?;

        // `graphql.resolve` returns `jsonb`. `row.get::<_, String>(0)` — as this line read before —
        // uses `FromSql for String`, which only accepts TEXT-family OIDs and PANICS (not returns an
        // Err) on a JSONB column, taking the whole worker thread down with it. This is the same class
        // of bug as the `$2::jsonb` bind above, on the decode side: get it as `serde_json::Value`
        // directly, whose `FromSql` impl (same `with-serde_json-1` feature) accepts JSON/JSONB.
        let result: serde_json::Value = row.get(0);
        Ok(result)
    }
}
