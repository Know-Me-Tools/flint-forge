//! p16-c005 gate: in production mode (the default), a missing
//! `FLINT_GATE_AUDIENCE` fails closed instead of silently skipping the
//! audience check.
//!
//! One test per file (see `tests/common/mod.rs`) so the crate's process-global
//! JWKS cache and `FLINT_GATE_*` env vars never leak across test runs.
mod common;

#[tokio::test]
async fn missing_audience_fails_closed_in_production_mode() {
    // SAFETY (test-only): this binary runs this single test, so no other
    // thread races these env vars.
    unsafe {
        // No FLINT_GATE_MODE set at all — production is the default.
        std::env::remove_var("FLINT_GATE_MODE");
        std::env::remove_var("FLINT_GATE_AUDIENCE");
        std::env::set_var("FLINT_GATE_ISSUER", "https://gate.example.com");
        // Deliberately unreachable: the audience check must fail BEFORE any
        // JWKS network call, so this URL is never actually hit.
        std::env::set_var("FLINT_GATE_JWKS_URL", "http://127.0.0.1:1/unreachable");
    }

    let result = forge_identity::verify_and_build("not-even-a-real-jwt").await;

    match result {
        Err(forge_identity::IdentityError::MissingEnv(name)) => {
            assert_eq!(name, "FLINT_GATE_AUDIENCE");
        }
        other => panic!("expected MissingEnv(\"FLINT_GATE_AUDIENCE\"), got {other:?}"),
    }
}
