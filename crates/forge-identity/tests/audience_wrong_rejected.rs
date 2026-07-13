//! p16-c005 gate: in production mode, a token with the wrong `aud` claim is
//! rejected rather than silently accepted.
//!
//! One test per file (see `tests/common/mod.rs`) so the crate's process-global
//! JWKS cache and `FLINT_GATE_*` env vars never leak across test runs.
mod common;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn wrong_audience_token_is_rejected_in_production_mode() {
    const ISSUER: &str = "https://gate.example.com";
    const EXPECTED_AUDIENCE: &str = "flint-quarry";

    // SAFETY (test-only): this binary runs this single test, so no other
    // thread races these env vars.
    unsafe {
        // No FLINT_GATE_MODE set — production is the default, and the
        // audience check IS configured here (unlike
        // audience_missing_fails_closed.rs), so we reach real verification.
        std::env::remove_var("FLINT_GATE_MODE");
        std::env::set_var("FLINT_GATE_AUDIENCE", EXPECTED_AUDIENCE);
        std::env::set_var("FLINT_GATE_ISSUER", ISSUER);
    }

    let key = common::generate_es256_key("kid-1");
    let server = MockServer::start().await;
    unsafe {
        std::env::set_var("FLINT_GATE_JWKS_URL", format!("{}/jwks", server.uri()));
    }
    Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(common::jwks_response(&[&key])))
        .mount(&server)
        .await;

    let wrong_aud_token = common::make_token(&key, ISSUER, Some("some-other-audience"), "user-1");
    let result = forge_identity::verify_and_build(&wrong_aud_token).await;
    assert!(
        matches!(result, Err(forge_identity::IdentityError::Verification(_))),
        "expected a Verification error for the wrong audience, got {result:?}"
    );

    // Sanity check: the SAME key/issuer/audience config accepts a token with
    // the correct audience, proving the rejection above is really about the
    // audience mismatch and not a broken test fixture.
    let correct_aud_token = common::make_token(&key, ISSUER, Some(EXPECTED_AUDIENCE), "user-1");
    forge_identity::verify_and_build(&correct_aud_token)
        .await
        .expect("token with the correct audience verifies");
}
