//! Identity: JWT verification, RLS context assembly, and Option-3 outbound auth.
#![forbid(unsafe_code)]

pub mod error;
pub mod jwks;

pub use error::IdentityError;

use forge_domain::{Json, SubjectId, TenantId};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::jwks::get_jwks;

/// Decoded JWT claims minted by flint-gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    /// `role` is NOT auto-included by flint-gate; absent → coerced to "anon".
    #[serde(default)]
    pub role: Option<String>,
    pub tenant_id: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Json>,
}

/// Transaction-scoped row-level-security context applied to a pooled connection.
#[derive(Debug, Clone)]
pub struct RlsContext {
    pub role: String,
    pub claims_json: String,
    pub raw_bearer: String,
    /// Ory Keto subject ID derived from the JWT `sub` claim.
    /// Used for subscribe-time Keto coarse checks.
    /// MUST NOT appear in tracing spans or logs.
    pub keto_subject: String,
    /// Flint Vault key ID scoped to this tenant/session (optional).
    /// Present only when the JWT carries a `vault_key_id` claim.
    /// MUST NOT appear in tracing spans or logs.
    pub vault_key_id: Option<String>,
}

impl RlsContext {
    pub fn subject(&self) -> Option<SubjectId> {
        serde_json::from_str::<Claims>(&self.claims_json)
            .ok()
            .map(|c| SubjectId(c.sub))
    }
    pub fn tenant(&self) -> Option<TenantId> {
        serde_json::from_str::<Claims>(&self.claims_json)
            .ok()
            .and_then(|c| c.tenant_id)
            .map(TenantId)
    }
}

/// Verify a bearer JWT against flint-gate's issuer/JWKS and build an `RlsContext`.
///
/// # Environment variables
/// - `FLINT_GATE_JWKS_URL` — JWKS endpoint (e.g. `https://gate.example.com/.well-known/jwks.json`)
/// - `FLINT_GATE_ISSUER` — expected `iss` claim
/// - `FLINT_GATE_AUDIENCE` — expected `aud` claim (optional; validation skipped if absent)
///
/// # Security contract
/// - `role` absent from JWT → coerced to `"anon"` (NOT an error per jwt-contract.md)
/// - JWT signature is ALWAYS verified; Postgres never sees unverified tokens
/// - Raw bearer is stored in `RlsContext` for outbound forwarding by `flint_hooks`/`flint_llm`
///   — do NOT log it
#[instrument(skip(bearer), err)]
pub async fn verify_and_build(bearer: &str) -> Result<RlsContext, IdentityError> {
    let jwks_url = std::env::var("FLINT_GATE_JWKS_URL")
        .map_err(|_| IdentityError::MissingEnv("FLINT_GATE_JWKS_URL"))?;
    let issuer = std::env::var("FLINT_GATE_ISSUER")
        .map_err(|_| IdentityError::MissingEnv("FLINT_GATE_ISSUER"))?;
    let audience = std::env::var("FLINT_GATE_AUDIENCE").ok();

    let jwks = get_jwks(&jwks_url).await?;

    let header = decode_header(bearer).map_err(|_| IdentityError::InvalidToken)?;
    let kid = header.kid.ok_or(IdentityError::MissingKid)?;

    let jwk = jwks
        .find(&kid)
        .ok_or_else(|| IdentityError::UnknownKid(kid.clone()))?;

    let decoding_key =
        DecodingKey::from_jwk(jwk).map_err(|e| IdentityError::Verification(e.to_string()))?;

    let algorithm = match header.alg {
        Algorithm::RS256 => Algorithm::RS256,
        Algorithm::RS384 => Algorithm::RS384,
        Algorithm::RS512 => Algorithm::RS512,
        Algorithm::ES256 => Algorithm::ES256,
        Algorithm::ES384 => Algorithm::ES384,
        other => return Err(IdentityError::Verification(format!("unsupported algorithm: {other:?}"))),
    };

    let mut validation = Validation::new(algorithm);
    validation.set_issuer(&[&issuer]);
    if let Some(aud) = &audience {
        validation.set_audience(&[aud]);
    } else {
        validation.validate_aud = false;
    }

    let token_data = decode::<Claims>(bearer, &decoding_key, &validation)
        .map_err(|e| IdentityError::Verification(e.to_string()))?;

    let claims = token_data.claims;

    // role absent → "anon" per jwt-contract.md — this is NOT an error
    let role = claims.role.clone().unwrap_or_else(|| "anon".to_string());

    // keto_subject is the JWT `sub` claim (used for Keto relation checks)
    let keto_subject = claims.sub.clone();

    // vault_key_id is an optional claim that scopes Flint Vault key access
    let vault_key_id = claims
        .extra
        .get("vault_key_id")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);

    // Serialize the full claims for SET LOCAL "request.jwt.claims"
    let claims_json = serde_json::to_string(&claims)
        .map_err(|e| IdentityError::ClaimsSerialize(e.to_string()))?;

    Ok(RlsContext {
        role,
        claims_json,
        raw_bearer: bearer.to_string(),
        keto_subject,
        vault_key_id,
    })
}

/// Option-3 hybrid outbound headers: service bearer + origin JWT + HMAC signature.
pub fn outbound_headers(
    service_token: &str,
    origin_jwt: &str,
    signature: &str,
) -> Vec<(String, String)> {
    vec![
        ("authorization".into(), format!("Bearer {service_token}")),
        ("x-forge-origin-jwt".into(), origin_jwt.to_string()),
        ("x-forge-signature".into(), signature.to_string()),
    ]
}
