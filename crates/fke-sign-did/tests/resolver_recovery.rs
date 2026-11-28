//! A named publisher can publish its DID document after a failed resolution.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::{Signer, SigningKey};
use fke_domain::FunctionManifest;
use fke_ports::{SignError, SignatureVerifier};
use fke_sign_did::VerifierDid;
use serde_json::json;
use sha2::{Digest, Sha256};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn unavailable_keys_are_retried_and_validity_windows_accept_offsets() {
    let resolver = MockServer::start().await;
    std::env::set_var("FLINT_DID_RESOLVER_URL", resolver.uri());
    let verifier = VerifierDid::default();
    let artifact = b"release component";
    let mut manifest = FunctionManifest {
        publisher_did: "did:prometheus:publisher".into(),
        content_digest: format!("sha256:{:x}", Sha256::digest(artifact)),
        capabilities: vec![],
        version: "1.0.0".into(),
        signature_b64: None,
        not_before: "2020-01-01T04:00:00.123+04:00".into(),
        not_after: "2099-12-31T17:59:59.999-06:00".into(),
    };
    let request = || Mock::given(method("GET")).and(path("/v1/did/did:prometheus:publisher"));
    let pending = request()
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"verificationMethod": []})))
        .expect(1)
        .mount_as_scoped(&resolver)
        .await;
    let signing_key = SigningKey::from_bytes(&[19; 32]);
    let mut signed = Sha256::digest(artifact).to_vec();
    signed.extend_from_slice(manifest.content_digest.as_bytes());
    let signature = signing_key.sign(&signed).to_bytes();
    assert!(matches!(
        verifier.verify(&manifest, &signature, artifact).await,
        Err(SignError::Invalid)
    ));
    drop(pending);

    request().respond_with(ResponseTemplate::new(200).set_body_json(json!({
        "verificationMethod": [{"publicKeyBase64Url": URL_SAFE_NO_PAD.encode(signing_key.verifying_key().as_bytes())}]
    }))).expect(1).mount(&resolver).await;
    verifier
        .verify(&manifest, &signature, artifact)
        .await
        .expect("published key");
    manifest.not_after = "no timestamp".into();
    assert!(matches!(
        verifier.verify(&manifest, &signature, artifact).await,
        Err(SignError::Invalid)
    ));
}
