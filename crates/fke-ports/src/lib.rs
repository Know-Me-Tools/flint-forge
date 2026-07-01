//! Flint Kiln port traits.
#![forbid(unsafe_code)]

use async_trait::async_trait;
use fke_domain::{ContentId, FunctionManifest, TargetArch};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StoreError {
    #[error("not found")]
    NotFound,
    #[error("io: {0}")]
    Io(String),
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SignError {
    #[error("unsigned")]
    Unsigned,
    #[error("invalid signature")]
    Invalid,
    #[error("expired")]
    Expired,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CompileError {
    #[error("verify first")]
    Unverified,
    #[error("backend: {0}")]
    Backend(String),
}

#[async_trait]
pub trait ComponentStore: Send + Sync {
    async fn put(&self, bytes: &[u8]) -> Result<ContentId, StoreError>;
    async fn get(&self, id: &ContentId) -> Result<Vec<u8>, StoreError>;
    async fn exists(&self, id: &ContentId) -> Result<bool, StoreError>;
}

#[async_trait]
pub trait SignatureVerifier: Send + Sync {
    /// Verify signature + publisher DID + validity window before instantiation.
    async fn verify(
        &self,
        manifest: &FunctionManifest,
        signature: &[u8],
        artifact: &[u8],
    ) -> Result<(), SignError>;
}

#[async_trait]
pub trait Compiler: Send + Sync {
    /// Control-plane only. AOT-compile a verified component to native `.cwasm` for a target.
    async fn precompile(
        &self,
        artifact: &[u8],
        target: &TargetArch,
    ) -> Result<Vec<u8>, CompileError>;
}

#[async_trait]
pub trait ComponentRegistry: Send + Sync {
    async fn resolve(&self, name: &str, version: &str) -> Result<FunctionManifest, StoreError>;
}
