/// Errors produced by the reflection engine, pipeline passes, and compilers.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ReflectionError {
    /// A `flint_meta.*` catalog query (or the `flint_a2ui.components` load)
    /// failed at the `sqlx` level — connection lost, malformed row shape, etc.
    #[error("database query failed")]
    Query(#[from] sqlx::Error),
    /// The `passes::validation` pass rejected the reflected `DatabaseModel`
    /// (e.g. a structural invariant the compilers depend on does not hold).
    #[error("model validation failed: {0}")]
    Validation(String),
    /// One of the REST/GraphQL/MCP/A2UI compilers failed to build its output
    /// from an otherwise-valid `DatabaseModel`.
    #[error("compiler error: {0}")]
    Compiler(String),
}
