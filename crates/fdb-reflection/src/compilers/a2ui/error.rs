//! Error type and publisher trait for the A2UI assembler.

/// Errors produced by the A2UI assembler.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum AssemblerError {
    #[error("database query failed")]
    Database(#[from] sqlx::Error),

    #[error("no assembly rule matched and no default binding found for {0}.{1}")]
    NoBinding(String, String),

    #[error("invalid assembly configuration: {0}")]
    InvalidConfig(String),

    #[error("event payload missing required field {0}")]
    MissingField(String),
}

/// Optional publisher for assembled surfaces (e.g. FRF Iggy topic).
#[async_trait::async_trait]
pub trait A2uiPublisher: Send + Sync {
    /// Publish a serialized surface to the given topic.
    async fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), AssemblerError>;
}
