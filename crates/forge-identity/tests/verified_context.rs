//! Exercise the verified JWT context and its outbound forwarding contract.
mod common;

use forge_identity::{outbound_headers, verify_and_build, IdentityError};
use jsonwebtoken::{Algorithm, Header};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn verified_claims_survive_rls_and_outbound_forwarding() {
    const ISSUER: &str = "https://issuer.example";
    let server = MockServer::start().await;
    let key = common::generate_es256_key("context-key");
    // This integration binary owns its environment and process-global key cache.
    std::env::set_var("FLINT_GATE_ISSUER", ISSUER);
    std::env::set_var("FLINT_GATE_AUDIENCE", "quarry");
    std::env::set_var("FLINT_GATE_JWKS_URL", server.uri());
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(common::jwks_response(&[&key])))
        .mount(&server)
        .await;

    let expires = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_secs()
        + 3600;
    let claims = json!({
        "sub": "reader", "role": "authenticated", "tenant_id": "library",
        "exp": expires, "iss": ISSUER, "aud": "quarry",
        "vault_key_id": "catalog-key", "scopes": ["books"]
    });
    let mut header = Header::new(Algorithm::ES256);
    let missing_kid = jsonwebtoken::encode(&header, &claims, &key.encoding_key).expect("sign");
    assert!(matches!(
        verify_and_build(&missing_kid).await,
        Err(IdentityError::MissingKid)
    ));
    header.kid = Some(key.kid.clone());
    let token = jsonwebtoken::encode(&header, &claims, &key.encoding_key).expect("sign");
    let mut context = verify_and_build(&token).await.expect("verified context");
    assert_eq!(context.role, "authenticated");
    assert_eq!(context.subject().expect("subject").0, "reader");
    assert_eq!(context.tenant().expect("tenant").0, "library");
    assert_eq!(context.vault_key_id.as_deref(), Some("catalog-key"));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&context.claims_json).expect("claims"),
        claims
    );
    assert_eq!(context.raw_bearer, token);

    let headers: std::collections::BTreeMap<_, _> =
        outbound_headers("service-token", &context.raw_bearer, "signature")
            .into_iter()
            .collect();
    assert_eq!(headers.len(), 3);
    assert_eq!(headers["authorization"], "Bearer service-token");
    assert_eq!(headers["x-forge-origin-jwt"], token);
    assert_eq!(headers["x-forge-signature"], "signature");

    context.claims_json = "{}".into();
    assert!(context.subject().is_none());
    assert!(context.tenant().is_none());
}
