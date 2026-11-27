//! Authenticated file metadata persistence boundary.
use async_trait::async_trait;
use fdb_domain::files::{FileId, FileQuery, FileRecord, FileType, PutFile};
use forge_identity::RlsContext;

/// Stable public failure categories; never include database/identity details.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FileError {
    /// Unsupported schema or invalid request/attributes.
    #[error("invalid file metadata")]
    Invalid,
    /// Conflicting immutable definition, revision or retry key.
    #[error("file metadata conflict")]
    Conflict,
    /// No visible record exists.
    #[error("file metadata not found")]
    NotFound,
    /// Multiple definitions matched; caller must select explicitly.
    #[error("ambiguous file type")]
    Ambiguous,
    /// Missing authenticated tenant/subject or denied access.
    #[error("file metadata access denied")]
    Denied,
    /// Persistence unavailable; no success is asserted.
    #[error("file metadata unavailable")]
    Unavailable,
}

/// Every operation uses verified, transaction-local identity and forced RLS.
#[async_trait]
pub trait FileMetadataStore: Send + Sync {
    /// Publish a private immutable type version; identical replay succeeds.
    async fn put_type(&self, rls: &RlsContext, value: &FileType) -> Result<FileType, FileError>;
    /// List private type versions in stable type/version order.
    async fn types(&self, rls: &RlsContext) -> Result<Vec<FileType>, FileError>;
    /// Atomically validate, compare revision, mutate and persist replay receipt.
    async fn put(
        &self,
        rls: &RlsContext,
        id: &FileId,
        value: &PutFile,
    ) -> Result<FileRecord, FileError>;
    /// Read one private file instance.
    async fn get(&self, rls: &RlsContext, id: &FileId) -> Result<FileRecord, FileError>;
    /// Discover private instances with bounded containment predicates.
    async fn query(
        &self,
        rls: &RlsContext,
        query: &FileQuery,
    ) -> Result<Vec<FileRecord>, FileError>;
}
