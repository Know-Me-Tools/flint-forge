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

use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json};
use ed25519_dalek::{Signer as _, SigningKey};
use fke_domain::FunctionManifest;
use fke_registry::PgComponentStore;
use fke_runtime::EdgeRuntime;

use crate::handlers::admin::{list_functions, register_function, require_admin, RegisterBody};
use crate::handlers::invoke::invoke_impl;
use crate::state::{verify_manifest_signature, KilnState};

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
fn signed_manifest(
    signing_key: &SigningKey,
    artifact: &[u8],
    content_digest: &str,
) -> FunctionManifest {
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

// ─── p16-c003: mandatory auth on the data and control planes ──────────────
//
// These call the private handler functions directly (not through a mounted
// axum Router/TestClient) since the auth check is the FIRST thing each
// handler does, before any DB access — `test_state()`'s lazy, never-dialed
// pool is safe here for exactly the same reason it's safe in the p16-c002
// tests above.

/// p16-c003 task 11 (invalid-token variant): a syntactically-present but
/// unverifiable bearer is also rejected 401, not treated as anonymous.
#[tokio::test]
async fn invoke_with_garbage_bearer_is_rejected_401() {
    let state = test_state();
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::AUTHORIZATION,
        "Bearer not-a-real-jwt".parse().expect("header value"),
    );
    let resp = invoke_impl(
        &state,
        "some-function",
        "latest",
        headers,
        axum::body::Bytes::new(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// p16-c003 task 12: an anonymous call to `GET /admin/functions` is rejected
/// 401 — previously this route had no auth middleware at all.
#[tokio::test]
async fn anonymous_admin_list_is_rejected_401() {
    let state = test_state();
    let resp = list_functions(State(state), HeaderMap::new())
        .await
        .into_response();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// p16-c003 task 12 (register variant): an anonymous call to
/// `POST /admin/functions` is also rejected 401, before signature
/// verification or storage are ever reached.
#[tokio::test]
async fn anonymous_admin_register_is_rejected_401() {
    let state = test_state();
    let key = test_signing_key();
    let artifact = b"fake wasm bytes";
    let manifest = signed_manifest(&key, artifact, "sha256:test");
    let body = RegisterBody {
        name: "some-function".into(),
        version: "1.0.0".into(),
        manifest,
        wasm_base64: {
            use base64::Engine as _;
            base64::engine::general_purpose::STANDARD.encode(artifact)
        },
    };

    let resp = register_function(State(state), HeaderMap::new(), Json(body))
        .await
        .into_response();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// p16-c003 gate: an anonymous (no `Authorization` header) call to
/// `/functions/v1/<name>` is rejected with 401 before ever touching the
/// registry/store/runtime — `invoke_impl`'s bearer check is the very first
/// thing it does, so a lazy (never-dialed) pool in `test_state()` is safe.
#[tokio::test]
async fn invoke_without_bearer_is_401() {
    let state = test_state();
    let resp = invoke_impl(
        &state,
        "some-function",
        "latest",
        HeaderMap::new(),
        axum::body::Bytes::new(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// p16-c003 gate: an anonymous call to `/admin/functions` (either
/// `register_function` or `list_functions`, both gated by `require_admin`
/// first) is rejected with 401 — previously this route had no runtime auth
/// at all, gated only by the compile-time `control-plane` feature flag.
#[tokio::test]
async fn admin_route_without_bearer_is_401() {
    let resp = require_admin(&HeaderMap::new())
        .await
        .expect_err("missing bearer must be rejected");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// p16-c003 gate: a syntactically-present but invalid/unparseable bearer is
/// also 401 (not merely "missing header" — the JWT itself must verify).
#[tokio::test]
async fn admin_route_with_garbage_bearer_is_401() {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        "Bearer not-a-real-jwt".parse().unwrap(),
    );
    let resp = require_admin(&headers)
        .await
        .expect_err("invalid bearer must be rejected");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
