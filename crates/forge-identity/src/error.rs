//! Error type for JWT verification and JWKS cache operations.

/// Every way `forge-identity` can fail to verify a bearer token or maintain
/// the JWKS cache backing that verification.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum IdentityError {
    /// The bearer string could not be decoded as a JWT at all (malformed
    /// header/payload/signature segments).
    #[error("invalid token")]
    InvalidToken,
    /// The token was well-formed but failed cryptographic or claim
    /// validation (bad signature, wrong issuer/audience, expired, or an
    /// unsupported signing algorithm).
    #[error("verification failed: {0}")]
    Verification(String),
    /// The JWKS endpoint could not be reached, or responded with a
    /// non-success HTTP status.
    #[error("failed to fetch JWKS: {0}")]
    JwksFetch(String),
    /// The JWKS endpoint responded, but the body did not parse as a valid
    /// JSON Web Key Set.
    #[error("failed to parse JWKS: {0}")]
    JwksParse(String),
    /// The JWT header did not include a `kid`, so no signing key could be
    /// looked up in the JWKS set.
    #[error("JWT header missing `kid`")]
    MissingKid,
    /// The JWT's `kid` does not match any key in the (possibly just
    /// refreshed) JWKS set — the signing key may not exist, or the cache
    /// refetch was rate-limited before it could pick up a genuine rotation.
    #[error("unknown `kid`: {0}")]
    UnknownKid(String),
    /// The verified claims could not be re-serialized to JSON for
    /// `SET LOCAL "request.jwt.claims"`.
    #[error("failed to serialize claims: {0}")]
    ClaimsSerialize(String),
    /// A required environment variable (named in the payload) was unset,
    /// e.g. `FLINT_GATE_JWKS_URL`, `FLINT_GATE_ISSUER`, or
    /// `FLINT_GATE_AUDIENCE` in production mode.
    #[error("required environment variable not set: {0}")]
    MissingEnv(&'static str),
}
