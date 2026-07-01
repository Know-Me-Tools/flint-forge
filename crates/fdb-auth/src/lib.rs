//! JWT verification → RlsContext (delegates to forge-identity; owns the JWKS cache).
#![forbid(unsafe_code)]

use forge_identity::{IdentityError, RlsContext};

pub async fn rls_from_bearer(bearer: &str) -> Result<RlsContext, IdentityError> {
    forge_identity::verify_and_build(bearer).await
}
