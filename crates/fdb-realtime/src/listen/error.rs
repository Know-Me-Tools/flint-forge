/// Errors from `ListenChangeSource` construction.
///
/// Neither variant carries detail: the DSN can embed a password and `sqlx::Error`
/// may echo the connection string, so the underlying cause is logged once (redacted)
/// and never returned.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ListenError {
    /// The dedicated listener connection could not be established.
    #[error("listener connect")]
    Connect,
    /// `LISTEN` on the change channel failed.
    #[error("listen setup")]
    Listen,
}

/// Redact a `sqlx::Error` down to its variant discriminant so nothing derived from
/// the DSN (host, user, password) can reach a log line.
pub(super) fn redact(err: &sqlx::Error) -> &'static str {
    match err {
        sqlx::Error::Configuration(_) => "configuration",
        sqlx::Error::Io(_) => "io",
        sqlx::Error::Tls(_) => "tls",
        sqlx::Error::Protocol(_) => "protocol",
        sqlx::Error::PoolTimedOut => "pool-timeout",
        sqlx::Error::PoolClosed => "pool-closed",
        sqlx::Error::Database(_) => "database",
        _ => "other",
    }
}
