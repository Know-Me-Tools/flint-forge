//! JWT verification → RlsContext (delegates to forge-identity; owns the JWKS cache).
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use forge_identity::{IdentityError, RlsContext};

/// Verify a raw JWT bearer token (flint-gate issuer/JWKS) and build the
/// [`RlsContext`] Quarry sets on every pooled connection before any user
/// statement runs.
///
/// This is the single entry point Quarry's request pipeline calls per
/// request; Postgres itself never verifies the JWT signature.
///
/// # Errors
///
/// Returns [`IdentityError::InvalidToken`] when `bearer` is not a
/// well-formed JWT; [`IdentityError::Verification`] when the signature,
/// issuer, audience, expiry, or algorithm checks fail; and a JWKS-related
/// variant (`JwksFetch`/`JwksParse`) when the signing key set cannot be
/// fetched or parsed, or the token's `kid` cannot be resolved against it.
pub async fn rls_from_bearer(bearer: &str) -> Result<RlsContext, IdentityError> {
    forge_identity::verify_and_build(bearer).await
}
