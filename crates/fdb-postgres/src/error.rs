//! Error type internal to the `fdb-postgres` adapter.
//!
//! [`PgError`] is the error surfaced by [`crate::PgBackend`] and the executor
//! adapters (`PgRest`, `PgGraphQl`, `PgVectorRpc`) for failures specific to the
//! Postgres/deadpool layer. Every port trait method converts it to the
//! port-level `fdb_ports::BackendError` at the boundary (see the `From` impl
//! below) so callers outside this crate never depend on adapter-specific detail.

/// Errors raised inside the `fdb-postgres` adapter before conversion to the
/// port-level `fdb_ports::BackendError`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PgError {
    /// The deadpool `Pool` could not be configured — e.g. the `DATABASE_URL`
    /// environment variable is missing, or `deadpool_postgres::Config`
    /// rejected the connection string.
    #[error("pool configuration error: {0}")]
    Config(String),
    /// Checking out a connection `Object` from the pool failed (pool
    /// exhausted, backend refused the connection, etc.).
    #[error("pool checkout failed: {0}")]
    Checkout(String),
    /// The initial `BEGIN` statement that opens the per-request transaction
    /// (required before any `SET LOCAL`) raised a raw `tokio_postgres` error.
    #[error("transaction error: {0}")]
    Transaction(#[from] tokio_postgres::Error),
    /// Applying one of the six RLS/GUC context statements
    /// (`SET LOCAL ROLE` or a `set_config(...)` call) failed — either the
    /// underlying statement errored, or the role identifier failed the
    /// safe-identifier check before it was interpolated into SQL.
    #[error("SET LOCAL failed: {0}")]
    SetLocal(String),
}

/// Render a `tokio_postgres::Error` with the server's own message.
///
/// `tokio_postgres::Error`'s `Display` is deliberately terse — a connection
/// failure prints `"error connecting to server"` and *any* server-side failure
/// prints just `"db error"`. The actual `DbError` (severity, SQLSTATE, message,
/// detail, hint) is only reachable through [`std::error::Error::source`].
///
/// Without this, every failure inside `acquire` reaches the logs as
/// `query: db error` — indistinguishable between a missing GRANT, an RLS
/// denial, a syntax error, and a dropped connection. That turns a
/// one-line-to-diagnose permission problem into a bisect.
///
/// Errors are logged server-side only; callers still receive a generic
/// message, so this discloses nothing to a client.
pub(crate) fn describe_pg(e: &tokio_postgres::Error) -> String {
    use std::error::Error as _;
    use std::fmt::Write as _;

    match e.as_db_error() {
        Some(db) => {
            let mut msg = format!("{}: {}", db.code().code(), db.message());
            if let Some(detail) = db.detail() {
                let _ = write!(msg, " (detail: {detail})");
            }
            if let Some(hint) = db.hint() {
                let _ = write!(msg, " (hint: {hint})");
            }
            msg
        }
        // Not a server error (I/O, TLS, protocol). Walk the chain, since the
        // outer Display is uninformative there too.
        None => match e.source() {
            Some(source) => format!("{e}: {source}"),
            None => e.to_string(),
        },
    }
}

impl From<PgError> for fdb_ports::BackendError {
    fn from(e: PgError) -> Self {
        match e {
            PgError::Config(_) | PgError::Checkout(_) => fdb_ports::BackendError::Connection,
            PgError::Transaction(e) => fdb_ports::BackendError::Query(describe_pg(&e)),
            PgError::SetLocal(msg) => fdb_ports::BackendError::Query(msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `PgError::SetLocal` already carries its own text; conversion must not
    /// discard it.
    #[test]
    fn set_local_message_survives_conversion() {
        let err: fdb_ports::BackendError =
            PgError::SetLocal("SET LOCAL ROLE: permission denied".into()).into();
        assert!(err.to_string().contains("permission denied"));
    }

    /// Pool problems are connection-level, not query-level — callers retry
    /// those differently.
    #[test]
    fn pool_errors_map_to_connection() {
        let err: fdb_ports::BackendError = PgError::Checkout("pool exhausted".into()).into();
        assert!(matches!(err, fdb_ports::BackendError::Connection));

        let err: fdb_ports::BackendError = PgError::Config("bad DATABASE_URL".into()).into();
        assert!(matches!(err, fdb_ports::BackendError::Connection));
    }
}
