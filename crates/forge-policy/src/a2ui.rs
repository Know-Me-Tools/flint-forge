//! A2UI Cedar capability constants and request builders.
//!
//! These definitions mirror the `a2ui:*` action namespace used by the Flint
//! A2UI Component Registry permission model. They are intentionally simple
//! string constants + helpers so the rest of the policy engine stays agnostic
//! of any one subsystem.
#![forbid(unsafe_code)]

use crate::{Decision, Pep, Request};
use async_trait::async_trait;
use forge_identity::RlsContext;

/// Resource identifier for the A2UI component catalog.
pub const A2UI_RESOURCE: &str = "a2ui:components";

/// View (read/list) components in an application catalog.
pub const A2UI_VIEW: &str = "a2ui:view";

/// Register a new component or override in an application catalog.
pub const A2UI_REGISTER: &str = "a2ui:register";

/// Emit an assembled A2UI payload (e.g. AG-UI / A2UI protocol surface).
pub const A2UI_EMIT: &str = "a2ui:emit";

/// Build a Cedar [`Request`] for an A2UI capability check.
pub fn request(action: &str) -> Request {
    Request {
        action: action.into(),
        resource: A2UI_RESOURCE.into(),
        context: forge_domain::Json::Null,
    }
}

/// Convenience trait for checking A2UI capabilities against any [`Pep`].
#[async_trait]
pub trait A2UIPep {
    async fn can_view(&self, who: &RlsContext) -> Decision;
    async fn can_register(&self, who: &RlsContext) -> Decision;
    async fn can_emit(&self, who: &RlsContext) -> Decision;
}

#[async_trait]
impl<T: Pep + Sync> A2UIPep for T {
    async fn can_view(&self, who: &RlsContext) -> Decision {
        self.check(who, &request(A2UI_VIEW)).await
    }

    async fn can_register(&self, who: &RlsContext) -> Decision {
        self.check(who, &request(A2UI_REGISTER)).await
    }

    async fn can_emit(&self, who: &RlsContext) -> Decision {
        self.check(who, &request(A2UI_EMIT)).await
    }
}

// ─── unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_uses_a2ui_resource() {
        let r = request(A2UI_VIEW);
        assert_eq!(r.action, A2UI_VIEW);
        assert_eq!(r.resource, A2UI_RESOURCE);
    }

    #[test]
    fn capability_constants_are_distinct() {
        assert_ne!(A2UI_VIEW, A2UI_REGISTER);
        assert_ne!(A2UI_VIEW, A2UI_EMIT);
        assert_ne!(A2UI_REGISTER, A2UI_EMIT);
    }
}
