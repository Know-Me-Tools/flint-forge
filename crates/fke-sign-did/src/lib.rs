//! Ed25519 / did:prometheus signature verifier — sovereign default with HTTP fallback.
//!
//! Verifies WASM component artifacts signed with a `did:prometheus` DID.
//! The public key may be embedded inline in the DID string, or resolved via
//! an HTTP DID document endpoint with a TTL cache.
//!
//! # DID format — inline key
//!
//! ```text
//! did:prometheus:<base64url(ed25519_public_key_bytes)>
//! ```
//!
//! The DID encodes the 32-byte raw Ed25519 public key as URL-safe base64
//! (no padding). Example:
//! ```text
//! did:prometheus:47DEQpj8HBSa-_TImW-5JCeuQeRkm5NMpJWZG3hSuFU
//! ```
//!
//! # DID format — named (HTTP resolved)
//!
//! Any suffix that does not decode to exactly 32 bytes is treated as a named
//! DID.  Resolution is delegated to `{FLINT_DID_RESOLVER_URL}/v1/did/{did}`.
//!
//! # Signed message
//!
//! `message = sha256(artifact_bytes) || content_digest_bytes`
//!
//! where `content_digest_bytes = manifest.content_digest.as_bytes()`.
//!
//! # Validity window
//!
//! `manifest.not_before` and `manifest.not_after` are RFC 3339 timestamps.
//! Verification fails with `SignError::Expired` if the current UTC time falls
//! outside the window.
#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, VerifyingKey};
use fke_domain::FunctionManifest;
use fke_ports::{SignError, SignatureVerifier};
use sha2::{Digest, Sha256};

const DEFAULT_RESOLVER_URL: &str = "https://did.flint.example.com";
const DEFAULT_CACHE_TTL: Duration = Duration::from_mins(5);

// ─── Public type ────────────────────────────────────────────────────────────

/// Verifier using Ed25519 + `did:prometheus` DID documents.
///
/// Tries to decode the DID suffix as an inline 32-byte Ed25519 public key
/// first (no network).  Falls back to HTTP DID document resolution with an
/// in-memory TTL cache when the suffix is not a valid inline key.
pub struct VerifierDid {
    resolver_url: String,
    client: reqwest::Client,
    key_cache: Mutex<HashMap<String, (VerifyingKey, Instant)>>,
    cache_ttl: Duration,
}

impl VerifierDid {
    /// Create a new `VerifierDid`.
    ///
    /// Reads the resolver URL from the `FLINT_DID_RESOLVER_URL` environment
    /// variable; falls back to `https://did.flint.example.com`.
    pub fn new() -> Self {
        let url = std::env::var("FLINT_DID_RESOLVER_URL")
            .unwrap_or_else(|_| DEFAULT_RESOLVER_URL.to_owned());
        Self::with_resolver(url)
    }

    /// Create a `VerifierDid` with an explicit resolver base URL.
    ///
    /// Useful for tests that supply a mock HTTP server.
    pub fn with_resolver(url: impl Into<String>) -> Self {
        Self {
            resolver_url: url.into(),
            client: reqwest::Client::new(),
            key_cache: Mutex::new(HashMap::new()),
            cache_ttl: DEFAULT_CACHE_TTL,
        }
    }
}

impl Default for VerifierDid {
    fn default() -> Self {
        Self::new()
    }
}

// ─── SignatureVerifier impl ──────────────────────────────────────────────────

#[async_trait]
impl SignatureVerifier for VerifierDid {
    async fn verify(
        &self,
        manifest: &FunctionManifest,
        signature: &[u8],
        artifact: &[u8],
    ) -> Result<(), SignError> {
        // 1. Resolve DID → Ed25519 public key
        let verifying_key = self.resolve_key(&manifest.publisher_did).await?;

        // 2. Check validity window
        check_validity_window(&manifest.not_before, &manifest.not_after)?;

        // 3. Build message: sha256(artifact) || content_digest bytes
        let msg = build_message(artifact, &manifest.content_digest);

        // 4. Decode signature bytes → ed25519 Signature
        let sig = Signature::from_slice(signature).map_err(|_| SignError::Invalid)?;

        // 5. Verify
        verifying_key
            .verify_strict(&msg, &sig)
            .map_err(|_| SignError::Invalid)
    }
}

// ─── Key resolution ─────────────────────────────────────────────────────────

impl VerifierDid {
    /// Resolve the Ed25519 key for a `did:prometheus:` DID.
    ///
    /// Fast path: if the suffix is a valid 32-byte base64url Ed25519 key, no
    /// network call is made.  Slow path: TTL-cached HTTP GET to the resolver.
    async fn resolve_key(&self, did: &str) -> Result<VerifyingKey, SignError> {
        let suffix = did
            .strip_prefix("did:prometheus:")
            .ok_or(SignError::Invalid)?;

        if suffix.is_empty() {
            return Err(SignError::Invalid);
        }

        // Fast path — inline key
        if let Ok(key) = try_inline_key(suffix) {
            return Ok(key);
        }

        // Slow path — check cache first (never hold the lock across an await)
        {
            let cache = self.key_cache.lock().map_err(|_| SignError::Invalid)?;
            if let Some((key, ts)) = cache.get(did) {
                if ts.elapsed() < self.cache_ttl {
                    return Ok(*key);
                }
            }
        }

        // HTTP fetch
        let key = self.fetch_key_http(did).await?;

        // Store in cache
        if let Ok(mut cache) = self.key_cache.lock() {
            cache.insert(did.to_owned(), (key, Instant::now()));
        }

        Ok(key)
    }

    /// Fetch the verifying key from the HTTP DID resolver.
    async fn fetch_key_http(&self, did: &str) -> Result<VerifyingKey, SignError> {
        let url = format!("{}/v1/did/{}", self.resolver_url, did);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|_| SignError::Invalid)?;

        if !response.status().is_success() {
            return Err(SignError::Invalid);
        }

        let doc: DidDocument = response.json().await.map_err(|_| SignError::Invalid)?;

        let vm = doc
            .verification_method
            .into_iter()
            .next()
            .ok_or(SignError::Invalid)?;

        let key_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            &vm.public_key_base64_url,
        )
        .map_err(|_| SignError::Invalid)?;

        let key_array: [u8; 32] = key_bytes.try_into().map_err(|_| SignError::Invalid)?;
        VerifyingKey::from_bytes(&key_array).map_err(|_| SignError::Invalid)
    }
}

// ─── JSON types ─────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct DidDocument {
    #[serde(rename = "verificationMethod")]
    verification_method: Vec<VerificationMethod>,
}

#[derive(serde::Deserialize)]
struct VerificationMethod {
    #[serde(rename = "publicKeyBase64Url")]
    public_key_base64_url: String,
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Attempt to decode the DID suffix as an inline 32-byte Ed25519 key.
fn try_inline_key(encoded: &str) -> Result<VerifyingKey, SignError> {
    let key_bytes =
        base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, encoded)
            .map_err(|_| SignError::Invalid)?;

    let key_array: [u8; 32] = key_bytes.try_into().map_err(|_| SignError::Invalid)?;
    VerifyingKey::from_bytes(&key_array).map_err(|_| SignError::Invalid)
}

/// Check that current UTC time is inside `[not_before, not_after]`.
fn check_validity_window(not_before: &str, not_after: &str) -> Result<(), SignError> {
    let now = Utc::now();

    let nb = DateTime::parse_from_rfc3339(not_before)
        .map_err(|_| SignError::Invalid)?
        .with_timezone(&Utc);

    let na = DateTime::parse_from_rfc3339(not_after)
        .map_err(|_| SignError::Invalid)?
        .with_timezone(&Utc);

    if now < nb || now > na {
        return Err(SignError::Expired);
    }

    Ok(())
}

/// Build the message to verify: `sha256(artifact) || content_digest_bytes`.
fn build_message(artifact: &[u8], content_digest: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(artifact);
    let artifact_hash: [u8; 32] = hasher.finalize().into();

    let mut msg = Vec::with_capacity(32 + content_digest.len());
    msg.extend_from_slice(&artifact_hash);
    msg.extend_from_slice(content_digest.as_bytes());
    msg
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
