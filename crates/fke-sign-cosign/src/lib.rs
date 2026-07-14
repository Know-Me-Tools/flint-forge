//! Sigstore/Cosign signature verifier — OCI registry interop.
//!
//! Verifies WASM artifacts signed via Cosign by querying the Rekor
//! transparency log for the artifact's sha256 digest.
//!
//! Two verification modes are selected by `FLINT_COSIGN_MODE`:
//!
//! * **`full`** (default) — full Sigstore chain-of-trust: the Rekor
//!   `publicKey.content` is parsed as a Fulcio-issued X.509 certificate, and
//!   all of the following must hold:
//!   1. **Chain** ([`chain`]) — the leaf's signature cryptographically
//!      verifies against the pinned Sigstore Fulcio intermediate CA, and the
//!      intermediate against the pinned root — not a string match on the
//!      issuer field. The certificate validity window is checked for the
//!      leaf, intermediate, and root.
//!   2. **SCT** ([`sct`]) — at least one of the leaf's embedded Signed
//!      Certificate Timestamps verifies against the pinned Sigstore CT log
//!      public key, per RFC 6962 §3.2. Without this, a compromised or
//!      misbehaving Fulcio instance's certs would still be accepted as long
//!      as they chain to the pinned root.
//!   3. **Identity** ([`identity`]) — if `FLINT_COSIGN_IDENTITY_ALLOWLIST` is
//!      configured, the leaf's embedded OIDC issuer/subject identity must
//!      match an allowlist entry. Unconfigured (the default) accepts any
//!      identity Fulcio was willing to issue a certificate for.
//!
//!   The `VerifyingKey` used for the *artifact* signature is derived from the
//!   leaf's `SubjectPublicKeyInfo` (P-256); chain verification itself is
//!   P-384 (the CA certs' curve).
//! * **`legacy`** — raw ECDSA P-256 only: `publicKey.content` is treated as
//!   SEC1-encoded key bytes with no certificate chain checks.
//!
//! # Scope boundaries
//!
//! Only the currently active (2022+) Fulcio CA generation and CT log
//! generation are pinned; a 2021–2022 predecessor generation (no separate
//! intermediate CA) is intentionally not accepted. SCT verification only
//! recognizes the sha256+ecdsa combination Sigstore actually issues.
//!
//! # Environment variables
//!
//! * `FLINT_REKOR_URL` — Rekor API base URL
//!   (default: `https://rekor.sigstore.dev`).
//! * `FLINT_COSIGN_MODE` — `"full"` (default) or `"legacy"`.
//! * `FLINT_COSIGN_IDENTITY_ALLOWLIST` — optional, `full` mode only. See
//!   [`identity`] for format.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod chain;
mod identity;
mod sct;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use fke_domain::FunctionManifest;
use fke_ports::{SignError, SignatureVerifier};
use p256::ecdsa::{signature::Verifier as _, DerSignature, VerifyingKey};
use sha2::{Digest, Sha256};
use x509_cert::{der::DecodePem, Certificate};

const DEFAULT_REKOR_URL: &str = "https://rekor.sigstore.dev";

// ─── Mode ────────────────────────────────────────────────────────────────────

/// Controls how `publicKey.content` from the Rekor log entry is interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerifierCosignMode {
    /// Full Sigstore verification: chain, SCT, and (if configured) identity
    /// allowlist — see the module docs.
    #[default]
    Full,
    /// Legacy path: treat `publicKey.content` as raw SEC1 key bytes.
    /// No certificate chain checks are performed.
    Legacy,
}

impl VerifierCosignMode {
    /// Read mode from `FLINT_COSIGN_MODE`.
    ///
    /// * `"full"` or absent → [`VerifierCosignMode::Full`]
    /// * `"legacy"` → [`VerifierCosignMode::Legacy`]
    pub fn from_env() -> Self {
        match std::env::var("FLINT_COSIGN_MODE")
            .as_deref()
            .unwrap_or("full")
        {
            "legacy" => Self::Legacy,
            _ => Self::Full,
        }
    }
}

// ─── Verifier ────────────────────────────────────────────────────────────────

/// Cosign verifier backed by the Rekor transparency log.
pub struct VerifierCosign {
    rekor_url: String,
    client: reqwest::Client,
    mode: VerifierCosignMode,
}

impl VerifierCosign {
    /// Create a verifier using environment variables for configuration.
    pub fn new() -> Self {
        let rekor_url =
            std::env::var("FLINT_REKOR_URL").unwrap_or_else(|_| DEFAULT_REKOR_URL.to_owned());
        Self {
            rekor_url,
            client: reqwest::Client::new(),
            mode: VerifierCosignMode::from_env(),
        }
    }

    /// Create a verifier with a custom Rekor URL (for air-gapped or test use).
    /// Mode is still read from `FLINT_COSIGN_MODE`.
    pub fn with_url(rekor_url: impl Into<String>) -> Self {
        Self {
            rekor_url: rekor_url.into(),
            client: reqwest::Client::new(),
            mode: VerifierCosignMode::from_env(),
        }
    }

    /// Create a verifier with explicit URL and mode (primarily for tests).
    pub fn with_url_and_mode(rekor_url: impl Into<String>, mode: VerifierCosignMode) -> Self {
        Self {
            rekor_url: rekor_url.into(),
            client: reqwest::Client::new(),
            mode,
        }
    }
}

impl Default for VerifierCosign {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SignatureVerifier for VerifierCosign {
    async fn verify(
        &self,
        manifest: &FunctionManifest,
        _sig: &[u8],
        artifact: &[u8],
    ) -> Result<(), SignError> {
        // 1. Manifest-level validity window.
        check_validity_window(&manifest.not_before, &manifest.not_after)?;

        // 2. Hash the artifact.
        let digest_hex = sha256_hex(artifact);

        // 3. Fetch Rekor log entry → (DER signature, raw publicKey bytes).
        let (sig_der, pubkey_bytes) = self.fetch_rekor_entry(&digest_hex).await?;

        // 4. Derive VerifyingKey according to the configured mode.
        let verifying_key = match self.mode {
            VerifierCosignMode::Full => extract_key_from_fulcio_cert(&pubkey_bytes)?,
            VerifierCosignMode::Legacy => {
                VerifyingKey::from_sec1_bytes(&pubkey_bytes).map_err(|_| SignError::Invalid)?
            }
        };

        // 5. Parse and verify the DER signature over sha256(artifact).
        let signature =
            DerSignature::try_from(sig_der.as_slice()).map_err(|_| SignError::Invalid)?;
        let digest_bytes = hex::decode(&digest_hex).map_err(|_| SignError::Invalid)?;
        verifying_key
            .verify(&digest_bytes, &signature)
            .map_err(|_| SignError::Invalid)
    }
}

// ─── Full-mode: Fulcio certificate chain + SCT + identity ───────────────────

/// Parse `pubkey_bytes` as PEM-encoded X.509, cryptographically verify the
/// chain to the pinned Sigstore root, verify an embedded SCT against the
/// pinned CT log, check the operator-configured identity allowlist (if any),
/// validate every cert's validity window, then extract the leaf's ECDSA
/// P-256 verifying key.
fn extract_key_from_fulcio_cert(pubkey_bytes: &[u8]) -> Result<VerifyingKey, SignError> {
    let pem_str = std::str::from_utf8(pubkey_bytes).map_err(|_| SignError::Invalid)?;
    let leaf = Certificate::from_pem(pem_str).map_err(|_| SignError::Invalid)?;

    let intermediate = chain::verify_chain_to_pinned_root(&leaf)?;
    chain::check_cert_validity(&leaf)?;
    sct::verify_embedded_scts(&leaf, &intermediate)?;
    identity::verify_identity_allowlist(&leaf)?;

    // `subject_public_key.raw_bytes()` returns the SEC1 EC point bytes.
    let key_bytes = leaf
        .tbs_certificate
        .subject_public_key_info
        .subject_public_key
        .raw_bytes();
    VerifyingKey::from_sec1_bytes(key_bytes).map_err(|_| SignError::Invalid)
}

// ─── Rekor entry fetch ───────────────────────────────────────────────────────

impl VerifierCosign {
    /// POST to Rekor `/api/v1/log/entries/retrieve` and return
    /// `(sig_der_bytes, pubkey_content_bytes)`.
    async fn fetch_rekor_entry(&self, digest_hex: &str) -> Result<(Vec<u8>, Vec<u8>), SignError> {
        let url = format!("{}/api/v1/log/entries/retrieve", self.rekor_url);
        let body = serde_json::json!({ "hashes": [format!("sha256:{digest_hex}")] });

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|_| SignError::Invalid)?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(SignError::Unsigned);
        }
        if !resp.status().is_success() {
            return Err(SignError::Unsigned);
        }

        let entries: serde_json::Value = resp.json().await.map_err(|_| SignError::Invalid)?;
        let entry = entries
            .as_array()
            .and_then(|a| a.first())
            .ok_or(SignError::Unsigned)?;

        // Decode the base64-encoded body JSON embedded in the entry.
        let body_b64 = entry
            .get("body")
            .and_then(|v| v.as_str())
            .ok_or(SignError::Invalid)?;
        let body_bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, body_b64)
                .map_err(|_| SignError::Invalid)?;
        let body_json: serde_json::Value =
            serde_json::from_slice(&body_bytes).map_err(|_| SignError::Invalid)?;

        // Extract signature (base64 DER) and publicKey (base64 raw bytes / PEM).
        let sig_b64 = body_json
            .pointer("/spec/signature/content")
            .and_then(|v| v.as_str())
            .ok_or(SignError::Invalid)?;
        let pubkey_b64 = body_json
            .pointer("/spec/signature/publicKey/content")
            .and_then(|v| v.as_str())
            .ok_or(SignError::Invalid)?;

        let sig_der = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, sig_b64)
            .map_err(|_| SignError::Invalid)?;

        let pubkey_bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, pubkey_b64)
                .map_err(|_| SignError::Invalid)?;

        Ok((sig_der, pubkey_bytes))
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    format!("{:x}", h.finalize())
}

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

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    fn make_manifest(not_before: &str, not_after: &str) -> FunctionManifest {
        FunctionManifest {
            publisher_did: "did:prometheus:test".into(),
            content_digest: "sha256:abc".into(),
            capabilities: vec![],
            version: "1.0.0".into(),
            not_before: not_before.into(),
            not_after: not_after.into(),
            signature_b64: None,
        }
    }

    /// Build a valid mock Rekor response body for the given sig+pubkey fields.
    fn mock_rekor_response(sig_b64: &str, pubkey_b64: &str) -> serde_json::Value {
        let body = json!({
            "spec": {
                "signature": {
                    "content": sig_b64,
                    "publicKey": { "content": pubkey_b64 }
                }
            }
        });
        let body_b64 =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, body.to_string());
        json!([{ "body": body_b64, "logIndex": 1 }])
    }

    /// Rekor returns 404 → `SignError::Unsigned`.
    #[tokio::test]
    async fn rekor_not_found_returns_unsigned() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/log/entries/retrieve"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let verifier = VerifierCosign::with_url(server.uri());
        let manifest = make_manifest("2020-01-01T00:00:00Z", "2099-12-31T23:59:59Z");
        let result = verifier.verify(&manifest, &[], b"artifact").await;
        assert!(
            matches!(result, Err(SignError::Unsigned)),
            "expected Unsigned, got {result:?}"
        );
    }

    /// Rekor returns an entry with a garbage signature → `SignError::Invalid`.
    /// In Full mode (default) the SEC1 bytes are not valid PEM, so the cert
    /// parse fails before signature verification, still yielding `Invalid`.
    #[tokio::test]
    async fn invalid_sig_bytes_returns_invalid() {
        let server = MockServer::start().await;
        let bad_sig = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            b"not-a-real-sig",
        );
        // Generate a real P-256 key for the publicKey field.
        let sk = p256::ecdsa::SigningKey::random(&mut rand_core::OsRng);
        let pubkey_b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            sk.verifying_key().to_sec1_bytes(),
        );
        let body = mock_rekor_response(&bad_sig, &pubkey_b64);
        Mock::given(method("POST"))
            .and(path("/api/v1/log/entries/retrieve"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let verifier = VerifierCosign::with_url(server.uri());
        let manifest = make_manifest("2020-01-01T00:00:00Z", "2099-12-31T23:59:59Z");
        let result = verifier.verify(&manifest, &[], b"artifact").await;
        assert!(
            matches!(result, Err(SignError::Invalid)),
            "expected Invalid, got {result:?}"
        );
    }

    /// Expired manifest validity window is caught before Rekor is called.
    #[tokio::test]
    async fn expired_validity_returns_expired_before_rekor_call() {
        let verifier = VerifierCosign::with_url("http://127.0.0.1:0");
        let manifest = make_manifest("2020-01-01T00:00:00Z", "2020-12-31T23:59:59Z");
        let result = verifier.verify(&manifest, &[], b"artifact").await;
        assert!(
            matches!(result, Err(SignError::Expired)),
            "expected Expired, got {result:?}"
        );
    }

    /// `sha256_hex` is deterministic.
    #[test]
    fn sha256_hex_is_deterministic() {
        assert_eq!(sha256_hex(b"hello"), sha256_hex(b"hello"));
        assert_ne!(sha256_hex(b"hello"), sha256_hex(b"world"));
    }

    /// `sha256_hex` produces a 64-character lowercase hex string.
    #[test]
    fn sha256_hex_length() {
        assert_eq!(sha256_hex(b"test").len(), 64);
    }

    /// Full mode rejects a Rekor entry whose cert doesn't cryptographically
    /// chain to the pinned Fulcio intermediate (a self-signed cert, in this
    /// case) → `SignError::Invalid`.
    #[tokio::test]
    async fn full_mode_rejects_cert_not_chaining_to_pinned_intermediate() {
        let server = MockServer::start().await;

        // A self-signed P-256 cert with CN=Test CA, O=Test Org — not signed
        // by the pinned Fulcio intermediate, so chain verification fails.
        let cert_pem = NON_FULCIO_CERT_PEM;
        let pubkey_b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            cert_pem.as_bytes(),
        );
        // Signature content is irrelevant — chain check fires first.
        let dummy_sig =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b"irrelevant");
        let body = mock_rekor_response(&dummy_sig, &pubkey_b64);
        Mock::given(method("POST"))
            .and(path("/api/v1/log/entries/retrieve"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let verifier = VerifierCosign::with_url_and_mode(server.uri(), VerifierCosignMode::Full);
        let manifest = make_manifest("2020-01-01T00:00:00Z", "2099-12-31T23:59:59Z");
        let result = verifier.verify(&manifest, &[], b"artifact").await;
        assert!(
            matches!(result, Err(SignError::Invalid)),
            "expected Invalid for a cert not chaining to the pinned intermediate, got {result:?}"
        );
    }

    /// `FLINT_COSIGN_MODE=legacy` selects the legacy (raw SEC1) path.
    #[test]
    fn mode_env_var_selects_legacy() {
        // Safety: single-threaded unit test; env mutation is acceptable here.
        std::env::set_var("FLINT_COSIGN_MODE", "legacy");
        let mode = VerifierCosignMode::from_env();
        std::env::remove_var("FLINT_COSIGN_MODE");
        assert_eq!(
            mode,
            VerifierCosignMode::Legacy,
            "FLINT_COSIGN_MODE=legacy must select Legacy"
        );
    }

    // ─── Test cert fixture ────────────────────────────────────────────────────

    /// Self-signed P-256 certificate. Issuer: CN=Test CA, O=Test Org.
    /// Valid 2026-07-06 → 2126-06-12. Not signed by the pinned Fulcio
    /// intermediate, so chain verification rejects it regardless of its
    /// issuer DN text.
    ///
    /// Generated with:
    /// ```text
    /// openssl ecparam -name prime256v1 -genkey -noout -out test.key
    /// openssl req -new -x509 -key test.key -days 36500 \
    ///   -subj "/CN=Test CA/O=Test Org" -out test.cert
    /// ```
    const NON_FULCIO_CERT_PEM: &str = "\
-----BEGIN CERTIFICATE-----\n\
MIIBoTCCAUegAwIBAgIUOt5VSkox7LSCoKEIHthZaYlUoDowCgYIKoZIzj0EAwIw\n\
JTEQMA4GA1UEAwwHVGVzdCBDQTERMA8GA1UECgwIVGVzdCBPcmcwIBcNMjYwNzA2\n\
MTMyMzU0WhgPMjEyNjA2MTIxMzIzNTRaMCUxEDAOBgNVBAMMB1Rlc3QgQ0ExETAP\n\
BgNVBAoMCFRlc3QgT3JnMFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEcz7vCItk\n\
jg7XwyQVWcbdAM+W1uF5Y0MZW/lKjno1EyWauopz7YB7/7FjHjX/bz+xlatmwA+T\n\
OYSt2EU+xSiA0qNTMFEwHQYDVR0OBBYEFK0tnpdxEIiSaC/8xJf5rykbyPAaMB8G\n\
A1UdIwQYMBaAFK0tnpdxEIiSaC/8xJf5rykbyPAaMA8GA1UdEwEB/wQFMAMBAf8w\n\
CgYIKoZIzj0EAwIDSAAwRQIgKUdHyif75zcyPZZbbghLEeaJMD4Ju6cqYThIh5Mp\n\
kc8CIQCbk3TwsWrIMzgSa05maByH4l9B41J19dyAXxt6IbobNg==\n\
-----END CERTIFICATE-----\n";
}
