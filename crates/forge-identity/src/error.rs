#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum IdentityError {
    #[error("invalid token")]
    InvalidToken,
    #[error("verification failed: {0}")]
    Verification(String),
    #[error("failed to fetch JWKS: {0}")]
    JwksFetch(String),
    #[error("failed to parse JWKS: {0}")]
    JwksParse(String),
    #[error("JWT header missing `kid`")]
    MissingKid,
    #[error("unknown `kid`: {0}")]
    UnknownKid(String),
    #[error("failed to serialize claims: {0}")]
    ClaimsSerialize(String),
    #[error("required environment variable not set: {0}")]
    MissingEnv(&'static str),
}
