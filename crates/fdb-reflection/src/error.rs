/// Errors produced by the reflection engine, pipeline passes, and compilers.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ReflectionError {
    #[error("database query failed")]
    Query(#[from] sqlx::Error),
    #[error("model validation failed: {0}")]
    Validation(String),
    #[error("compiler error: {0}")]
    Compiler(String),
}
