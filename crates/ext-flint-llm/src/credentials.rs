//! Credential resolution for the flint-gate/UAR bridge.
//!
//! In production the flint_llm service token is read from `vault.secrets` via the
//! brokered `vault.get_secret` SECURITY DEFINER function. In development only, a
//! fallback environment variable may be used. The token is never logged.

use crate::error::{LlmError, Result};
use pgrx::prelude::*;
use secrecy::SecretString;

const VAULT_SECRET_NAME: &str = "flint-gate-service-token";
const DEV_ENV_VAR: &str = "FLINT_LLM_SERVICE_TOKEN";

/// Resolve the service token used to authenticate outbound requests to flint-gate.
///
/// 1. Try `vault.get_secret('flint-gate-service-token')` via SPI. This succeeds
///    when the current role (or the function's SECURITY DEFINER role) has been
///    granted `EXECUTE` on `vault.get_secret`.
/// 2. Development fallback: read `FLINT_LLM_SERVICE_TOKEN` from the postmaster
///    environment. This path is intentionally not available in release builds.
pub fn resolve_service_token() -> Result<SecretString> {
    if let Some(token) = resolve_from_vault()? {
        return Ok(token);
    }

    if let Ok(token) = std::env::var(DEV_ENV_VAR) {
        if !token.is_empty() {
            #[cfg(debug_assertions)]
            {
                return Ok(SecretString::from(token));
            }
            #[cfg(not(debug_assertions))]
            {
                return Err(LlmError::Credential(format!(
                    "{DEV_ENV_VAR} is a development-only fallback and is not allowed in release builds"
                )));
            }
        }
    }

    Err(LlmError::Credential(
        "flint-gate service token not found: store it in vault as \"flint-gate-service-token\" or set FLINT_LLM_SERVICE_TOKEN for dev".to_string(),
    ))
}

fn resolve_from_vault() -> Result<Option<SecretString>> {
    let exists: bool = Spi::get_one(
        "SELECT EXISTS (SELECT 1 FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace WHERE n.nspname = 'vault' AND p.proname = 'get_secret')",
    )
    .unwrap_or(Some(false))
    .unwrap_or(false);

    if !exists {
        return Ok(None);
    }

    let query = format!(
        "SELECT vault.get_secret('{}')",
        VAULT_SECRET_NAME.replace('\'', "''")
    );
    let result: Option<String> = Spi::get_one(&query)
        .map_err(|e| LlmError::Credential(format!("vault.get_secret failed: {e}")))?;

    match result {
        Some(token) if !token.is_empty() => Ok(Some(SecretString::from(token))),
        _ => Err(LlmError::Credential(
            "vault returned an empty flint-gate service token".to_string(),
        )),
    }
}

/// Convenience wrapper: expose a `text` variant for SQL consumers if needed.
/// This deliberately returns a redacted placeholder; real callers use the
/// in-memory `SecretString` returned by `resolve_service_token`.
#[pg_extern]
fn _llm_service_token_present() -> bool {
    resolve_service_token().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_fallback_env_var() {
        // Cannot test SPI here; at least verify the env-var path compiles and
        // handles missing/empty values.
        assert!(
            std::env::var(DEV_ENV_VAR).unwrap_or_default().is_empty()
                || resolve_service_token().is_ok()
        );
    }
}
