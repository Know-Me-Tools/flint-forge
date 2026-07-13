//! p16-c005 gate: an unknown `kid` triggers exactly one rate-limited refetch,
//! not a refetch-per-lookup storm.
//!
//! One test per file (see `tests/common/mod.rs`) so the crate's process-global
//! JWKS cache never leaks state across test runs.
mod common;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn refetch_on_unknown_kid_is_rate_limited() {
    // SAFETY (test-only): this binary runs this single test, so no other
    // thread races these env vars.
    unsafe {
        std::env::set_var("FLINT_GATE_MODE", "development");
        std::env::remove_var("FLINT_GATE_AUDIENCE");
        std::env::set_var("FLINT_GATE_ISSUER", "https://gate.example.com");
        // A long TTL so this test proves the unknown-kid fast path, not the
        // ordinary TTL refresh path (covered by jwks_rotation.rs).
        std::env::set_var("FLINT_GATE_JWKS_TTL_SECS", "3600");
    }

    let key_a = common::generate_es256_key("kid-a");
    let key_b = common::generate_es256_key("kid-b");
    let key_c = common::generate_es256_key("kid-c");

    let server = MockServer::start().await;
    unsafe {
        std::env::set_var("FLINT_GATE_JWKS_URL", format!("{}/jwks", server.uri()));
    }

    // Request 1: initial fetch, publishes only key A.
    let mock_a = Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(common::jwks_response(&[&key_a])))
        .mount_as_scoped(&server)
        .await;

    let token_a = common::make_token(&key_a, "https://gate.example.com", None, "user-1");
    forge_identity::verify_and_build(&token_a)
        .await
        .expect("token signed by key A verifies against the initial JWKS");

    // Rotate to key B. The cached JWKS (key A only) makes kid-b unknown, so
    // this should trigger request 2: the rate-limited refetch-on-unknown-kid.
    drop(mock_a);
    let mock_b = Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(common::jwks_response(&[&key_b])))
        .mount_as_scoped(&server)
        .await;

    let token_b = common::make_token(&key_b, "https://gate.example.com", None, "user-1");
    forge_identity::verify_and_build(&token_b)
        .await
        .expect("unknown kid triggers a refetch that picks up key B");

    // Rotate again to key C, immediately (well within the rate-limit window).
    // A second unknown-kid lookup this soon after the first refetch must NOT
    // hit the network again — it should fail against the still-stale
    // (key-B-only) cache instead of causing a second refetch.
    drop(mock_b);
    Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(common::jwks_response(&[&key_c])))
        .mount(&server)
        .await;

    let token_c = common::make_token(&key_c, "https://gate.example.com", None, "user-1");
    let result = forge_identity::verify_and_build(&token_c).await;
    assert!(
        result.is_err(),
        "rate-limited refetch must not have picked up key C yet"
    );

    let requests = server
        .received_requests()
        .await
        .expect("request recording is enabled by default");
    assert_eq!(
        requests.len(),
        2,
        "expected exactly 2 JWKS fetches (initial + one rate-limited refetch), got {}",
        requests.len()
    );
}
