//! p16-c002 gate tests: the signature-verification gate itself
//! (`verify_manifest_signature`), exercised directly rather than through a
//! full HTTP register/invoke round trip.
//!
//! `fke-server` is a pure binary crate (no `lib.rs`), so these live here —
//! not in `tests/` — to reach private items, matching the existing
//! `kiln_bgw.rs`/`kiln_policy.rs` convention of in-crate test modules.
//!
//! A full end-to-end HTTP test (register a real signed `.wasm` component,
//! invoke it, assert a real WASI-HTTP response) additionally requires the
//! `cargo-component` toolchain to build `examples/hello-component` and a live
//! Postgres with `flint_kiln.*` migrated — neither is available in this
//! environment. `fke-runtime`'s own gate tests
//! (`gate_hello_component_returns_http_200`) already self-skip for the same
//! reason; this module does not attempt what that toolchain gap blocks.
//! `register_function`/`invoke_impl`'s DB-touching paths are therefore not
//! covered here — only the signature gate they both call before touching
//! storage.

use super::*;
use ed25519_dalek::{Signer as _, SigningKey};
use fke_domain::FunctionManifest;

fn test_state() -> KilnState {
    // `connect_lazy` never dials — `verify_manifest_signature` only touches
    // `state.verifier_did`/`state.verifier_cosign`, never `state.store`/
    // `state.registry`, so a lazy (unreachable) pool is safe here.
    let pool = sqlx::PgPool::connect_lazy("postgres://localhost/unused").expect("lazy pool");
    KilnState {
        runtime: Arc::new(EdgeRuntime::new().expect("EdgeRuntime::new")),
        store: Arc::new(PgComponentStore::new(pool.clone())),
        registry: Arc::new(fke_registry::PgRegistry::new(pool)),
        verifier_did: Arc::new(fke_sign_did::VerifierDid::new()),
        verifier_cosign: Arc::new(fke_sign_cosign::VerifierCosign::new()),
    }
}

/// Build a `did:prometheus:` manifest signed correctly over `artifact`,
/// matching `fke-sign-did::VerifierDid`'s exact message construction
/// (`sha256(artifact) || content_digest.as_bytes()`).
fn signed_manifest(signing_key: &SigningKey, artifact: &[u8], content_digest: &str) -> FunctionManifest {
    use base64::Engine as _;
    use sha2::{Digest, Sha256};

    let did = format!(
        "did:prometheus:{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(signing_key.verifying_key().to_bytes())
    );

    let artifact_hash = Sha256::digest(artifact);
    let mut msg = Vec::with_capacity(32 + content_digest.len());
    msg.extend_from_slice(&artifact_hash);
    msg.extend_from_slice(content_digest.as_bytes());
    let signature = signing_key.sign(&msg);
    let signature_b64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());

    FunctionManifest {
        publisher_did: did,
        content_digest: content_digest.to_owned(),
        capabilities: vec![],
        version: "1.0.0".to_owned(),
        not_before: "2020-01-01T00:00:00Z".to_owned(),
        not_after: "2099-12-31T23:59:59Z".to_owned(),
        signature_b64: Some(signature_b64),
    }
}

fn test_signing_key() -> SigningKey {
    // Deterministic fixed seed — no RNG dependency in the test.
    SigningKey::from_bytes(&[7u8; 32])
}

/// p16-c002 task 10: an unsigned `did:prometheus:` manifest is rejected
/// before it ever reaches storage.
#[tokio::test]
async fn unsigned_component_rejected() {
    let state = test_state();
    let key = test_signing_key();
    let artifact = b"fake wasm bytes";
    let mut manifest = signed_manifest(&key, artifact, "sha256:test");
    manifest.signature_b64 = None; // strip the signature — this is the case under test

    let result = verify_manifest_signature(&state, &manifest, artifact).await;
    assert!(
        matches!(result, Err(fke_ports::SignError::Unsigned)),
        "expected Unsigned, got {result:?}"
    );
}

/// p16-c002 task 11: a validly signed manifest is rejected when the artifact
/// bytes presented for verification don't match what was actually signed
/// (tampered after signing, or signed-then-swapped).
#[tokio::test]
async fn tampered_artifact_rejected() {
    let state = test_state();
    let key = test_signing_key();
    let original_artifact = b"fake wasm bytes";
    let manifest = signed_manifest(&key, original_artifact, "sha256:test");

    let tampered_artifact = b"fake wasm BYTES"; // one byte-case flip
    let result = verify_manifest_signature(&state, &manifest, tampered_artifact).await;
    assert!(
        matches!(result, Err(fke_ports::SignError::Invalid)),
        "expected Invalid for tampered artifact, got {result:?}"
    );
}

/// p16-c002 task 12 (signature-gate portion): a correctly signed manifest
/// verifies successfully against the artifact it was actually signed over.
/// Full end-to-end WASM instantiation is not covered here — see module doc.
#[tokio::test]
async fn validly_signed_component_verifies() {
    let state = test_state();
    let key = test_signing_key();
    let artifact = b"fake wasm bytes";
    let manifest = signed_manifest(&key, artifact, "sha256:test");

    let result = verify_manifest_signature(&state, &manifest, artifact).await;
    assert!(result.is_ok(), "expected Ok, got {result:?}");
}
