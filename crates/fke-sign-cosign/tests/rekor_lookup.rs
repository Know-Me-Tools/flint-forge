//! Manifest time validation and error classification at the Rekor HTTP boundary.
use fke_domain::FunctionManifest;
use fke_ports::{SignError, SignatureVerifier};
use fke_sign_cosign::VerifierCosign;
use serde_json::json;
use sha2::{Digest, Sha256};
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn verifies_manifest_times_and_looks_up_the_exact_artifact_digest() {
    let rekor = MockServer::start().await;
    std::env::set_var("FLINT_REKOR_URL", rekor.uri());
    std::env::set_var("FLINT_COSIGN_MODE", "full");
    let verifier = VerifierCosign::default();
    let artifact = b"registered component";
    let mut manifest = FunctionManifest {
        publisher_did: "did:prometheus:publisher".into(),
        version: "1".into(),
        capabilities: vec![],
        content_digest: format!("sha256:{:x}", Sha256::digest(artifact)),
        signature_b64: None,
        not_before: "2020-01-01T01:15:00.000+01:15".into(),
        not_after: "no timestamp".into(),
    };
    assert!(matches!(
        verifier.verify(&manifest, &[], artifact).await,
        Err(SignError::Invalid)
    ));
    assert!(rekor
        .received_requests()
        .await
        .expect("recording")
        .is_empty());
    manifest.not_after = "2099-12-31T20:59:59.999-03:00".into();
    for (reply, unsigned) in [
        (ResponseTemplate::new(200).set_body_json(json!([])), true),
        (ResponseTemplate::new(503), true),
        (ResponseTemplate::new(200).set_body_json(json!([{}])), false),
        (
            ResponseTemplate::new(200).set_body_string("incomplete response"),
            false,
        ),
    ] {
        let mock = Mock::given(method("POST"))
            .and(path("/api/v1/log/entries/retrieve"))
            .and(body_json(json!({"hashes":[manifest.content_digest]})))
            .respond_with(reply)
            .expect(1)
            .mount_as_scoped(&rekor)
            .await;
        let result = verifier.verify(&manifest, &[], artifact).await;
        if unsigned {
            assert!(matches!(result, Err(SignError::Unsigned)));
        } else {
            assert!(matches!(result, Err(SignError::Invalid)));
        }
        drop(mock);
    }
}
