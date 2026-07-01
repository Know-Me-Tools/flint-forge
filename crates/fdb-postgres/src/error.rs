#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PgError {
    #[error("pool configuration error: {0}")]
    Config(String),
    #[error("pool checkout failed: {0}")]
    Checkout(String),
    #[error("transaction error: {0}")]
    Transaction(#[from] tokio_postgres::Error),
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
