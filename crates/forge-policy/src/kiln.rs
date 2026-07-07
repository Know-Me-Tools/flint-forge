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

/// Convenience trait for checking Kiln capabilities against any [`Pep`].
#[async_trait]
pub trait KilnPep {
    async fn can_invoke(&self, who: &RlsContext) -> Decision;
    async fn can_register(&self, who: &RlsContext) -> Decision;
}

#[async_trait]
impl<T: Pep + Sync> KilnPep for T {
    async fn can_invoke(&self, who: &RlsContext) -> Decision {
        self.check(who, &request(KILN_INVOKE)).await
    }

    async fn can_register(&self, who: &RlsContext) -> Decision {
        self.check(who, &request(KILN_REGISTER)).await
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
}
