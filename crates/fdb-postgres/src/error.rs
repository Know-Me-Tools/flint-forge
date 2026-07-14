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

impl From<PgError> for fdb_ports::BackendError {
    fn from(e: PgError) -> Self {
        match e {
            PgError::Config(_) | PgError::Checkout(_) => fdb_ports::BackendError::Connection,
            PgError::Transaction(e) => fdb_ports::BackendError::Query(e.to_string()),
            PgError::SetLocal(msg) => fdb_ports::BackendError::Query(msg),
        }
    }
}
