use super::DEFAULT_BROADCAST_CAPACITY;

/// Configuration for the in-process LISTEN/NOTIFY change source.
#[derive(Debug, Clone)]
pub struct ListenConfig {
    /// Postgres connection string for the DEDICATED listener connection.
    /// This is NOT a request-scoped pooled conn — `PgListener` holds it for the
    /// process lifetime. Use a low-privilege role; it only runs `LISTEN`.
    pub database_url: String,
    /// Bounded capacity of the broadcast channel feeding subscribers. Governs
    /// lag tolerance: a subscriber more than this many events behind is
    /// `Lagged` and skips events (never blocks the producer).
    pub broadcast_capacity: usize,
}

impl ListenConfig {
    /// Construct a config with the default broadcast capacity.
    #[must_use]
    pub fn new(database_url: impl Into<String>) -> Self {
        Self {
            database_url: database_url.into(),
            broadcast_capacity: DEFAULT_BROADCAST_CAPACITY,
        }
    }
}
