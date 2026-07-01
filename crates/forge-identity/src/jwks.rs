//! JWKS cache — fetches and caches flint-gate's JSON Web Key Set.
//!
//! Uses `tokio::sync::OnceCell` so the JWKS is fetched once per process lifetime.
//! A reconnect is required to rotate keys in production (acceptable for now).

use std::sync::OnceLock;

use jsonwebtoken::jwk::JwkSet;
use reqwest::Client;

use crate::error::IdentityError;

static JWKS: OnceLock<JwkSet> = OnceLock::new();
static HTTP: OnceLock<Client> = OnceLock::new();

fn http_client() -> &'static Client {
    HTTP.get_or_init(|| {
        Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("reqwest client construction is infallible with defaults")
    })
}

/// Fetch and cache the JWKS from `jwks_url`.
/// Subsequent calls return the cached value immediately.
pub async fn get_jwks(jwks_url: &str) -> Result<&'static JwkSet, IdentityError> {
    if let Some(cached) = JWKS.get() {
        return Ok(cached);
    }

    let response = http_client()
        .get(jwks_url)
        .send()
        .await
        .map_err(|e| IdentityError::JwksFetch(e.to_string()))?;

    if !response.status().is_success() {
        return Err(IdentityError::JwksFetch(format!(
            "JWKS endpoint returned {}",
            response.status()
        )));
    }

    let jwk_set: JwkSet = response
        .json()
        .await
        .map_err(|e| IdentityError::JwksParse(e.to_string()))?;

    // OnceLock::set returns Err if already set (race); either value is correct.
    let _ = JWKS.set(jwk_set);
    Ok(JWKS.get().expect("just set above"))
}
