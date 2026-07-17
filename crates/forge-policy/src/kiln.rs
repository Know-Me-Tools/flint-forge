//! Kiln Cedar capability constants and request builders.
//!
//! Mirrors `forge-policy/src/a2ui.rs` for the Kiln WASM edge-function
//! permission model. The `kiln:invoke` action gates every function
//! instantiation in `fke-runtime`; Cedar policy controls which
//! publishers/callers may invoke which functions.
#![forbid(unsafe_code)]

use crate::{Decision, Pep, Request};
use async_trait::async_trait;
use forge_identity::RlsContext;

/// Resource identifier for Kiln edge functions.
pub const KILN_RESOURCE: &str = "kiln:functions";

/// Invoke a Kiln edge function (data-plane execution).
pub const KILN_INVOKE: &str = "kiln:invoke";

/// Register a new function or update an existing one (control-plane write).
pub const KILN_REGISTER: &str = "kiln:register";

/// Build a Cedar [`Request`] for a Kiln capability check.
pub fn request(action: &str) -> Request {
    Request {
        action: action.into(),
        resource: KILN_RESOURCE.into(),
        context: forge_domain::Json::Null,
    }
}

/// Build the Cedar action name for a per-capability grant check
/// (`kiln:capability:<name>`, e.g. `kiln:capability:secrets`).
///
/// `name` is the lowercase capability identifier — see `fke_domain::Capability::as_str`.
pub fn capability_action(name: &str) -> String {
    format!("kiln:capability:{name}")
}

/// Action name for revealing a specific secret's plaintext.
pub const KILN_SECRET_REVEAL: &str = "kiln:secret:reveal";

/// Build a Cedar [`Request`] for revealing one named secret — finer-grained
/// than the interface-level `kiln:capability:secrets` check (which only
/// gates whether the `secrets` interface is reachable at all), scoped to the
/// secret itself as the resource so policy can grant per-secret access.
pub fn secret_reveal_request(secret_name: &str) -> Request {
    Request {
        action: KILN_SECRET_REVEAL.into(),
        resource: format!("kiln:secret:{secret_name}"),
        context: forge_domain::Json::Null,
    }
}

/// Convenience trait for checking Kiln capabilities against any [`Pep`].
#[async_trait]
pub trait KilnPep {
    /// Check whether `who` may invoke a Kiln edge function (data-plane
    /// execution).
    async fn can_invoke(&self, who: &RlsContext) -> Decision;
    /// Check whether `who` may register a new function or update an
    /// existing one (control-plane write).
    async fn can_register(&self, who: &RlsContext) -> Decision;
    /// Check whether `who` may use a specific `flint:host` capability
    /// (`db`, `llm`, `kv`, `identity`, `secrets`, `http_outgoing`) that a
    /// component's manifest declares. Distinct from `can_invoke`, which gates
    /// whether the caller may run the function at all.
    async fn can_use_capability(&self, who: &RlsContext, capability: &str) -> Decision;
}

#[async_trait]
impl<T: Pep + Sync> KilnPep for T {
    async fn can_invoke(&self, who: &RlsContext) -> Decision {
        self.check(who, &request(KILN_INVOKE)).await
    }

    async fn can_register(&self, who: &RlsContext) -> Decision {
        self.check(who, &request(KILN_REGISTER)).await
    }

    async fn can_use_capability(&self, who: &RlsContext, capability: &str) -> Decision {
        self.check(who, &request(&capability_action(capability)))
            .await
    }
}

// ─── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_uses_kiln_resource() {
        let r = request(KILN_INVOKE);
        assert_eq!(r.action, KILN_INVOKE);
        assert_eq!(r.resource, KILN_RESOURCE);
    }

    #[test]
    fn capability_constants_are_distinct() {
        assert_ne!(KILN_INVOKE, KILN_REGISTER);
        assert_ne!(KILN_RESOURCE, KILN_INVOKE);
    }

    #[test]
    fn capability_action_formats_kiln_capability_prefix() {
        assert_eq!(capability_action("secrets"), "kiln:capability:secrets");
        assert_ne!(capability_action("db"), capability_action("llm"));
    }

    #[test]
    fn secret_reveal_request_scopes_resource_to_secret_name() {
        let r = secret_reveal_request("db-password");
        assert_eq!(r.action, KILN_SECRET_REVEAL);
        assert_eq!(r.resource, "kiln:secret:db-password");
        assert_ne!(
            secret_reveal_request("a").resource,
            secret_reveal_request("b").resource
        );
    }
}
