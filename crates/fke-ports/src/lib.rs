//! Flint Kiln port traits.
//!
//! The trait seams Kiln's use-cases are written against: content-addressed
//! artifact storage (`ComponentStore`), signature/publisher verification
//! (`SignatureVerifier`), AOT compilation (`Compiler`), and function
//! resolution (`ComponentRegistry`). Concrete adapters live in
//! `fke-store-{oci,ipfs,s3,fs}`, `fke-sign-{did,cosign}`, and `fke-registry`;
//! per the hexagonal dependency rule this crate must never depend on any of
//! them.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use async_trait::async_trait;
use fke_domain::{ContentId, FunctionManifest, TargetArch};

/// Errors from a [`ComponentStore`] or [`ComponentRegistry`] backend.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StoreError {
    /// No artifact or function registration exists for the requested key.
    #[error("not found")]
    NotFound,
    /// The backend (filesystem, OCI registry, IPFS node, S3, Postgres, …)
    /// failed; the string carries the backend's own error message for
    /// diagnostics/logging (never the raw error type, to keep this port
    /// backend-agnostic).
    #[error("io: {0}")]
    Io(String),
}

/// Errors from a [`SignatureVerifier`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SignError {
    /// The component's [`FunctionManifest`] carries no signature for its
    /// publisher scheme (`signature_b64` is `None` for a `did:prometheus:`
    /// publisher, or no Rekor entry exists for a cosign-signed one).
    /// Unsigned components must be rejected at register and invoke.
    #[error("unsigned")]
    Unsigned,
    /// Signature verification failed: the signature does not match the
    /// artifact/content digest, the publisher key could not be resolved, or
    /// the manifest is otherwise malformed.
    #[error("invalid signature")]
    Invalid,
    /// The current time falls outside the manifest's
    /// `[not_before, not_after]` validity window.
    #[error("expired")]
    Expired,
}

/// Errors from a [`Compiler`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CompileError {
    /// `precompile` was called on an artifact that has not passed
    /// [`SignatureVerifier::verify`] yet — a caller contract violation, since
    /// only verified components may reach the control-plane compiler.
    #[error("verify first")]
    Unverified,
    /// The underlying wasmtime/Cranelift compilation failed (e.g. the
    /// artifact is not valid WASM, or uses features unsupported for
    /// `target`); the string carries the backend's own error message.
    #[error("backend: {0}")]
    Backend(String),
}

/// Content-addressed store for compiled WASM component artifacts.
///
/// Implementations are backend adapters (`fke-store-fs`, `fke-store-oci`,
/// `fke-store-ipfs`, `fke-store-s3`); the address space is
/// [`ContentId`] (a sha256 digest or IPFS CID) so the same artifact stored
/// twice always resolves to the same key.
#[async_trait]
pub trait ComponentStore: Send + Sync {
    /// Store `bytes` and return the [`ContentId`] they hash to.
    ///
    /// Writing the same bytes twice must be idempotent — implementations
    /// should treat an existing object at the derived content address as
    /// success, not an error.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Io`] if the backend cannot persist the bytes
    /// (permission denied, network failure, storage quota, etc.).
    async fn put(&self, bytes: &[u8]) -> Result<ContentId, StoreError>;

    /// Fetch the raw bytes previously stored under `id`.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::NotFound`] if no artifact exists for `id`, or
    /// [`StoreError::Io`] if the backend fails while reading.
    async fn get(&self, id: &ContentId) -> Result<Vec<u8>, StoreError>;

    /// Check whether an artifact is stored under `id` without fetching it.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Io`] if the backend fails while checking
    /// existence. A missing artifact is a successful `Ok(false)`, not an
    /// error.
    async fn exists(&self, id: &ContentId) -> Result<bool, StoreError>;
}

/// Verifies a component's signature, publisher identity, and validity window
/// before it may be registered or invoked.
///
/// Implementations are keyed to a publisher DID scheme: `fke-sign-did`
/// verifies `did:prometheus:` (Ed25519) signatures, `fke-sign-cosign`
/// verifies cosign/Sigstore signatures resolved from the Rekor transparency
/// log.
#[async_trait]
pub trait SignatureVerifier: Send + Sync {
    /// Verify signature + publisher DID + validity window before instantiation.
    ///
    /// # Errors
    ///
    /// Returns [`SignError::Unsigned`] if `manifest` carries no signature for
    /// its scheme, [`SignError::Invalid`] if the signature does not verify
    /// against `artifact`/the resolved publisher key, or
    /// [`SignError::Expired`] if the current time is outside
    /// `manifest.not_before`/`manifest.not_after`.
    async fn verify(
        &self,
        manifest: &FunctionManifest,
        signature: &[u8],
        artifact: &[u8],
    ) -> Result<(), SignError>;
}

/// Ahead-of-time compiles a verified WASM component to native code for a
/// target architecture.
///
/// Control-plane only per spec §5.2: the invocation (data-plane) server is
/// built with Cranelift/Winch disabled and never implements or calls this
/// trait — it only deserializes the `.cwasm` bytes this trait produces.
#[async_trait]
pub trait Compiler: Send + Sync {
    /// Control-plane only. AOT-compile a verified component to native `.cwasm` for a target.
    ///
    /// # Errors
    ///
    /// Returns [`CompileError::Unverified`] if called on an artifact that has
    /// not passed [`SignatureVerifier::verify`], or [`CompileError::Backend`]
    /// if the underlying compilation (e.g. Cranelift) fails for `target`.
    async fn precompile(
        &self,
        artifact: &[u8],
        target: &TargetArch,
    ) -> Result<Vec<u8>, CompileError>;
}

/// Resolves a registered function name and version to its signed
/// [`FunctionManifest`].
///
/// The concrete adapter (`fke-registry`) looks this up in
/// `flint_kiln.functions`; the manifest returned carries the `content_digest`
/// needed to fetch the artifact bytes from a [`ComponentStore`].
#[async_trait]
pub trait ComponentRegistry: Send + Sync {
    /// Resolve `name`@`version` to its registered, signed manifest.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::NotFound`] if no active registration matches
    /// `name` and `version`, or [`StoreError::Io`] if the backend fails while
    /// looking it up.
    async fn resolve(&self, name: &str, version: &str) -> Result<FunctionManifest, StoreError>;
}
