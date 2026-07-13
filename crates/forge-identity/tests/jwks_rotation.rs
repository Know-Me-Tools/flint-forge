//! p16-c005 gate: a JWKS key rotation is picked up without a process restart.
//!
//! One test per file (see `tests/common/mod.rs`) so the crate's process-global
//! JWKS cache never leaks state across test runs.
mod common;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn jwks_rotation_is_picked_up_without_restart() {
    // SAFETY (test-only): this binary runs this single test, so no other
    // thread races these env vars.
    unsafe {
        std::env::set_var("FLINT_GATE_MODE", "development");
        std::env::remove_var("FLINT_GATE_AUDIENCE");
        std::env::set_var("FLINT_GATE_ISSUER", "https://gate.example.com");
        // Force every get_jwks() call to treat the cache as stale, so the
        // rotation is proven via the TTL path (not the unknown-kid path,
        // which is covered by its own dedicated test).
        std::env::set_var("FLINT_GATE_JWKS_TTL_SECS", "0");
    }

    let key_a = common::generate_es256_key("kid-a");
    let key_b = common::generate_es256_key("kid-b");

    let server = MockServer::start().await;
    unsafe {
        std::env::set_var("FLINT_GATE_JWKS_URL", format!("{}/jwks", server.uri()));
    }

    let jwks_mock = Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(common::jwks_response(&[&key_a])))
        .expect(1..)
        .mount_as_scoped(&server)
        .await;

    let token_a = common::make_token(&key_a, "https://gate.example.com", None, "user-1");
    forge_identity::verify_and_build(&token_a)
        .await
        .expect("token signed by the initially-published key verifies");

    // Rotate: the upstream JWKS now only publishes key B. No restart, no
    // process-level state reset — just a new HTTP response.
    drop(jwks_mock);
    Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(common::jwks_response(&[&key_b])))
        .mount(&server)
        .await;

    let token_b = common::make_token(&key_b, "https://gate.example.com", None, "user-1");
    forge_identity::verify_and_build(&token_b)
        .await
        .expect("token signed by the rotated key verifies without a process restart");
}
