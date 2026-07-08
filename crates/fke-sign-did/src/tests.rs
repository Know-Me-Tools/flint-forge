use super::*;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

// ─── Shared helpers ──────────────────────────────────────────────────────────

fn make_test_keypair() -> (SigningKey, String) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    let did = format!(
        "did:prometheus:{}",
        base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            verifying_key.as_bytes()
        )
    );
    (signing_key, did)
}

fn make_manifest(did: &str, not_before: &str, not_after: &str) -> FunctionManifest {
    FunctionManifest {
        publisher_did: did.to_owned(),
        content_digest: "sha256:testdigest".to_owned(),
        capabilities: vec![],
        version: "1.0.0".to_owned(),
        not_before: not_before.to_owned(),
        not_after: not_after.to_owned(),
    }
}

fn sign_artifact(signing_key: &SigningKey, artifact: &[u8], content_digest: &str) -> Vec<u8> {
    use ed25519_dalek::Signer;
    let msg = build_message(artifact, content_digest);
    signing_key.sign(&msg).to_bytes().to_vec()
}

// ─── Signature verification ───────────────────────────────────────────────────

#[tokio::test]
async fn valid_signature_verifies() {
    let (sk, did) = make_test_keypair();
    let artifact = b"wasm bytes here";
    let manifest = make_manifest(&did, "2020-01-01T00:00:00Z", "2099-12-31T23:59:59Z");
    let sig = sign_artifact(&sk, artifact, &manifest.content_digest);
    assert!(VerifierDid::new()
        .verify(&manifest, &sig, artifact)
        .await
        .is_ok());
}

#[tokio::test]
async fn expired_not_after_returns_expired() {
    let (sk, did) = make_test_keypair();
    let artifact = b"wasm bytes here";
    let manifest = make_manifest(&did, "2020-01-01T00:00:00Z", "2020-12-31T23:59:59Z");
    let sig = sign_artifact(&sk, artifact, &manifest.content_digest);
    assert!(matches!(
        VerifierDid::new().verify(&manifest, &sig, artifact).await,
        Err(SignError::Expired)
    ));
}

#[tokio::test]
async fn not_yet_valid_returns_expired() {
    let (sk, did) = make_test_keypair();
    let artifact = b"wasm bytes here";
    let manifest = make_manifest(&did, "2099-01-01T00:00:00Z", "2099-12-31T23:59:59Z");
    let sig = sign_artifact(&sk, artifact, &manifest.content_digest);
    assert!(matches!(
        VerifierDid::new().verify(&manifest, &sig, artifact).await,
        Err(SignError::Expired)
    ));
}

#[tokio::test]
async fn wrong_signature_returns_invalid() {
    let (sk, did) = make_test_keypair();
    let manifest = make_manifest(&did, "2020-01-01T00:00:00Z", "2099-12-31T23:59:59Z");
    let sig = sign_artifact(&sk, b"different artifact", &manifest.content_digest);
    assert!(matches!(
        VerifierDid::new()
            .verify(&manifest, &sig, b"actual artifact")
            .await,
        Err(SignError::Invalid)
    ));
}

#[tokio::test]
async fn malformed_did_returns_invalid() {
    let manifest = make_manifest("not-a-did", "2020-01-01T00:00:00Z", "2099-12-31T23:59:59Z");
    assert!(matches!(
        VerifierDid::with_resolver("http://unused")
            .verify(&manifest, b"sig", b"artifact")
            .await,
        Err(SignError::Invalid)
    ));
}

#[tokio::test]
async fn short_key_in_did_returns_invalid() {
    let mock_server = wiremock::MockServer::start().await;
    let short_key = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        b"tooshort",
    );
    let manifest = make_manifest(
        &format!("did:prometheus:{short_key}"),
        "2020-01-01T00:00:00Z",
        "2099-12-31T23:59:59Z",
    );
    assert!(matches!(
        VerifierDid::with_resolver(mock_server.uri())
            .verify(&manifest, b"sig", b"artifact")
            .await,
        Err(SignError::Invalid)
    ));
}

#[test]
fn build_message_is_deterministic() {
    let a = build_message(b"artifact", "sha256:abc");
    let b = build_message(b"artifact", "sha256:abc");
    assert_eq!(a, b);
    assert_eq!(a.len(), 32 + "sha256:abc".len());
}

#[test]
fn build_message_differs_for_different_inputs() {
    assert_ne!(
        build_message(b"artifact1", "sha256:abc"),
        build_message(b"artifact2", "sha256:abc")
    );
}

// ─── HTTP resolution ──────────────────────────────────────────────────────────

#[tokio::test]
async fn inline_key_still_works_without_network() {
    let (sk, did) = make_test_keypair();
    let artifact = b"wasm bytes here";
    let manifest = make_manifest(&did, "2020-01-01T00:00:00Z", "2099-12-31T23:59:59Z");
    let sig = sign_artifact(&sk, artifact, &manifest.content_digest);
    assert!(
        VerifierDid::with_resolver("http://this-host-does-not-exist.invalid")
            .verify(&manifest, &sig, artifact)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn http_resolution_fetches_key_and_caches() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;
    let sk = SigningKey::generate(&mut OsRng);
    let pub_key_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        sk.verifying_key().as_bytes(),
    );
    let did = "did:prometheus:acme-corp-2024";
    let body = serde_json::json!({
        "verificationMethod": [{"type":"Ed25519VerificationKey2020","publicKeyBase64Url":pub_key_b64}]
    });
    Mock::given(method("GET"))
        .and(path(format!("/v1/did/{did}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .expect(1)
        .mount(&mock_server)
        .await;

    let verifier = VerifierDid::with_resolver(mock_server.uri());
    let artifact = b"wasm bytes here";
    let manifest = make_manifest(did, "2020-01-01T00:00:00Z", "2099-12-31T23:59:59Z");
    let sig = sign_artifact(&sk, artifact, &manifest.content_digest);

    assert!(
        verifier.verify(&manifest, &sig, artifact).await.is_ok(),
        "first call"
    );
    assert!(
        verifier.verify(&manifest, &sig, artifact).await.is_ok(),
        "second (cached)"
    );
}

#[tokio::test]
async fn http_resolution_404_returns_invalid() {
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;
    let manifest = make_manifest(
        "did:prometheus:non-existent",
        "2020-01-01T00:00:00Z",
        "2099-12-31T23:59:59Z",
    );
    assert!(matches!(
        VerifierDid::with_resolver(mock_server.uri())
            .verify(&manifest, b"sig", b"artifact")
            .await,
        Err(SignError::Invalid)
    ));
}

#[tokio::test]
async fn cached_key_returned_without_second_request() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let mock_server = MockServer::start().await;
    let sk = SigningKey::generate(&mut OsRng);
    let pub_key_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        sk.verifying_key().as_bytes(),
    );
    let did = "did:prometheus:cached-signer";
    let body = serde_json::json!({
        "verificationMethod": [{"type":"Ed25519VerificationKey2020","publicKeyBase64Url":pub_key_b64}]
    });
    Mock::given(method("GET"))
        .and(path(format!("/v1/did/{did}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .expect(1)
        .mount(&mock_server)
        .await;

    let verifier = VerifierDid::with_resolver(mock_server.uri());
    let artifact = b"data";
    let manifest = make_manifest(did, "2020-01-01T00:00:00Z", "2099-12-31T23:59:59Z");
    let sig = sign_artifact(&sk, artifact, &manifest.content_digest);
    verifier
        .verify(&manifest, &sig, artifact)
        .await
        .expect("first");
    verifier
        .verify(&manifest, &sig, artifact)
        .await
        .expect("second (cached)");
}
