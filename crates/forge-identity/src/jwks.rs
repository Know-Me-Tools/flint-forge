//! JWKS cache — fetches and caches flint-gate's JSON Web Key Set.
//!
//! Lock-free hot-reload via `ArcSwapOption` (same pattern as
//! `forge-policy::CedarPolicyEngine` / `fdb-reflection`'s `SchemaRegistry`),
//! with two refresh paths:
//!
//! - **TTL**: [`get_jwks`] refetches once the cached entry is older than
//!   [`jwks_ttl`] (default 10 minutes, `FLINT_GATE_JWKS_TTL_SECS` to override).
//! - **Unknown-`kid`**: [`refetch_on_unknown_kid`] force-refreshes when a
//!   verification fails because the signing key isn't in the cached set (the
//!   fast path for an unplanned upstream rotation), rate-limited so a burst of
//!   unknown-`kid` lookups (misconfigured client, or an attack) can't turn
//!   into a refetch storm against the JWKS endpoint.

use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use arc_swap::ArcSwapOption;
use jsonwebtoken::jwk::JwkSet;
use reqwest::Client;
use tokio::sync::Mutex;

use crate::error::IdentityError;

const DEFAULT_TTL_SECS: u64 = 600;
const MIN_REFETCH_INTERVAL: Duration = Duration::from_secs(5);

struct CachedJwks {
    set: Arc<JwkSet>,
    fetched_at: Instant,
}

static JWKS: OnceLock<ArcSwapOption<CachedJwks>> = OnceLock::new();
static HTTP: OnceLock<Client> = OnceLock::new();
static LAST_UNKNOWN_KID_REFETCH: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();

fn cache() -> &'static ArcSwapOption<CachedJwks> {
    JWKS.get_or_init(ArcSwapOption::empty)
}

fn http_client() -> &'static Client {
    HTTP.get_or_init(|| {
        Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("reqwest client construction is infallible with defaults")
    })
}

fn last_unknown_kid_refetch() -> &'static Mutex<Option<Instant>> {
    LAST_UNKNOWN_KID_REFETCH.get_or_init(|| Mutex::new(None))
}

/// TTL for the JWKS cache. Configurable via `FLINT_GATE_JWKS_TTL_SECS`
/// (falls back to `DEFAULT_TTL_SECS` on unset or unparseable values).
fn jwks_ttl() -> Duration {
    std::env::var("FLINT_GATE_JWKS_TTL_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map_or(Duration::from_secs(DEFAULT_TTL_SECS), Duration::from_secs)
}

async fn fetch(jwks_url: &str) -> Result<JwkSet, IdentityError> {
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

    response
        .json()
        .await
        .map_err(|e| IdentityError::JwksParse(e.to_string()))
}

async fn refresh(jwks_url: &str) -> Result<Arc<JwkSet>, IdentityError> {
    let set = Arc::new(fetch(jwks_url).await?);
    cache().store(Some(Arc::new(CachedJwks {
        set: Arc::clone(&set),
        fetched_at: Instant::now(),
    })));
    Ok(set)
}

/// Fetch and cache the JWKS from `jwks_url`, refreshing when the cache is
/// empty or older than [`jwks_ttl`].
pub async fn get_jwks(jwks_url: &str) -> Result<Arc<JwkSet>, IdentityError> {
    if let Some(cached) = cache().load_full() {
        if cached.fetched_at.elapsed() < jwks_ttl() {
            return Ok(Arc::clone(&cached.set));
        }
    }
    refresh(jwks_url).await
}

/// Force a JWKS refresh after a verification failed with an unknown `kid` —
/// the fast path for an unplanned upstream key rotation.
///
/// Rate-limited to [`MIN_REFETCH_INTERVAL`]: if called again within that
/// window, returns the existing cache (even if it still lacks the new key)
/// instead of hitting the network again, so a burst of unknown-`kid` lookups
/// can't turn into a refetch storm.
pub async fn refetch_on_unknown_kid(jwks_url: &str) -> Result<Arc<JwkSet>, IdentityError> {
    let mut last = last_unknown_kid_refetch().lock().await;
    if let Some(t) = *last {
        if t.elapsed() < MIN_REFETCH_INTERVAL {
            if let Some(cached) = cache().load_full() {
                return Ok(Arc::clone(&cached.set));
            }
            return Err(IdentityError::JwksFetch(
                "refetch rate-limited and no JWKS cached yet".to_string(),
            ));
        }
    }
    *last = Some(Instant::now());
    drop(last);
    refresh(jwks_url).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jwks_ttl_defaults_when_env_unset() {
        std::env::remove_var("FLINT_GATE_JWKS_TTL_SECS");
        assert_eq!(jwks_ttl(), Duration::from_secs(DEFAULT_TTL_SECS));
    }
}
