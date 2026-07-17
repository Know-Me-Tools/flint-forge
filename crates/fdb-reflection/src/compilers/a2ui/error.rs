//! Error type and publisher trait for the A2UI assembler.

/// Errors produced by the A2UI assembler.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum AssemblerError {
    /// A query against `flint_a2ui.assembly_rules`, `flint_a2ui.components`,
    /// or `flint_a2ui.bindings` failed at the `sqlx`/database level.
    #[error("database query failed")]
    Database(#[from] sqlx::Error),

    /// No application-specific assembly rule matched the event, and
    /// `flint_a2ui.bindings` has no default component binding for the
    /// `(schema, table)` named in the event's `data_source` — the event
    /// cannot be assembled into a surface.
    #[error("no assembly rule matched and no default binding found for {0}.{1}")]
    NoBinding(String, String),

    /// An assembly rule's `assembly_config` was malformed (missing
    /// `component_slug`/`component`, or named a component that does not
    /// exist in `flint_a2ui.components`), or surface serialization failed
    /// before publishing.
    #[error("invalid assembly configuration: {0}")]
    InvalidConfig(String),

    /// The event payload was missing a field the default binding path
    /// requires to resolve its data source — either the whole
    /// `data_source` object, or `data_source.table` within it.
    #[error("event payload missing required field {0}")]
    MissingField(String),
}

/// Optional publisher for assembled surfaces (e.g. FRF Iggy topic).
#[async_trait::async_trait]
pub trait A2uiPublisher: Send + Sync {
    /// Publish a serialized surface to the given topic.
    async fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), AssemblerError>;
}
