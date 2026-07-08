//! `flint:host/identity` — verified origin JWT context.
//!
//! The WIT `identity` interface exposes two functions injected by Flint Kiln
//! from the component's verified origin JWT:
//!
//! - `origin-jwt() -> option<string>` — the raw JWT, if the publisher allows
//!   the component to see it (default-deny).
//! - `claims()     -> string`         — JSON-encoded claim set, always present.
//!
//! Neither function returns signing-key material.

use serde_json::Value;

/// Identity context for Flint skills.
///
/// Implement this trait as a thin adapter over the WIT-generated
/// `bindings::flint::host::identity` module.
pub trait Identity {
    /// Return the raw origin JWT string, if the host made it available to
    /// this component. `None` means the publisher did not grant the component
    /// `identity.origin-jwt` access (default-deny).
    fn origin_jwt(&self) -> Option<String>;

    /// Return the JSON-encoded claim set. Always present — the host injects
    /// at minimum `iss`, `sub`, and `aud` from the verified JWT.
    fn claims_json(&self) -> String;

    /// Convenience: decode [`Identity::claims_json`] into a [`serde_json::Value`].
    ///
    /// Errors only if the host returned malformed JSON, which would be a host
    /// bug. Skill code that wants typed claims should define a `Claims` struct
    /// and call `serde_json::from_str` on [`Identity::claims_json`] directly.
    fn claims(&self) -> crate::error::SkillResult<Value> {
        let raw = self.claims_json();
        serde_json::from_str(&raw).map_err(|source| crate::SkillError::Json {
            source,
            payload: raw,
        })
    }
}
