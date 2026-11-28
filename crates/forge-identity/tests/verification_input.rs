//! Input and configuration failures are reported before looking up a signing key.
use forge_identity::{verify_and_build, IdentityError};

#[tokio::test]
async fn verifier_reports_missing_settings_then_malformed_input() {
    std::env::remove_var("FLINT_GATE_JWKS_URL");
    std::env::remove_var("FLINT_GATE_ISSUER");
    std::env::set_var("FLINT_GATE_AUDIENCE", "quarry");
    assert!(matches!(
        verify_and_build("incomplete").await,
        Err(IdentityError::MissingEnv("FLINT_GATE_JWKS_URL"))
    ));
    std::env::set_var("FLINT_GATE_JWKS_URL", "https://unused.invalid");
    assert!(matches!(
        verify_and_build("incomplete").await,
        Err(IdentityError::MissingEnv("FLINT_GATE_ISSUER"))
    ));
    std::env::set_var("FLINT_GATE_ISSUER", "https://issuer.example");
    assert!(matches!(
        verify_and_build("incomplete").await,
        Err(IdentityError::InvalidToken)
    ));
}
