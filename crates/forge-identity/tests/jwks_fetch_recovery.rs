//! A failed key fetch never installs an invalid cache entry.
mod common;

use forge_identity::jwks::{get_jwks, refetch_on_unknown_kid};
use forge_identity::IdentityError;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn empty_cache_recovers_after_unavailable_and_malformed_jwks() {
    let server = MockServer::start().await;
    let unavailable = format!("{}/unavailable", server.uri());
    Mock::given(method("GET"))
        .and(path("/unavailable"))
        .respond_with(ResponseTemplate::new(503))
        .expect(1)
        .mount(&server)
        .await;
    assert!(matches!(refetch_on_unknown_kid(&unavailable).await,
        Err(IdentityError::JwksFetch(reason)) if reason.contains("503")));
    assert!(matches!(refetch_on_unknown_kid(&unavailable).await,
        Err(IdentityError::JwksFetch(reason)) if reason.contains("rate-limited")));

    Mock::given(method("GET"))
        .and(path("/incomplete"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&server)
        .await;
    assert!(matches!(
        get_jwks(&format!("{}/incomplete", server.uri())).await,
        Err(IdentityError::JwksParse(_))
    ));

    let key = common::generate_es256_key("recovered");
    Mock::given(method("GET"))
        .and(path("/ready"))
        .respond_with(ResponseTemplate::new(200).set_body_json(common::jwks_response(&[&key])))
        .expect(1)
        .mount(&server)
        .await;
    let ready = format!("{}/ready", server.uri());
    std::env::set_var("FLINT_GATE_JWKS_TTL_SECS", "600");
    let published = get_jwks(&ready).await.expect("newly published key set");
    assert!(published.find("recovered").is_some());
    assert!(std::sync::Arc::ptr_eq(
        &published,
        &get_jwks(&ready).await.expect("cached set")
    ));
}
