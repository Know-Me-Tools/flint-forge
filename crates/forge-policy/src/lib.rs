//! Cedar policy enforcement point (PEP) shared by Quarry mutations, the Kiln linker, and Ember.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod a2ui;
pub mod cedar;
pub mod kiln;

pub use a2ui::{request, A2UIPep, A2UI_EMIT, A2UI_REGISTER, A2UI_RESOURCE, A2UI_VIEW};
pub use cedar::{CedarPolicyEngine, PolicyEntry, PolicyLoadError, PolicySource};
pub use kiln::{KilnPep, KILN_INVOKE, KILN_REGISTER, KILN_RESOURCE};

use async_trait::async_trait;
use forge_identity::RlsContext;

/// The outcome of evaluating a [`Request`] against policy: fail-closed
/// throughout `forge-policy`, so every construction/evaluation error path
/// maps to [`Decision::Deny`] rather than [`Decision::Allow`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// The request is permitted.
    Allow,
    /// The request is not permitted (including all fail-closed error paths).
    Deny,
}

/// A capability or action request evaluated against Cedar policy.
#[derive(Debug, Clone)]
pub struct Request {
    /// The Cedar action being requested, e.g. `"a2ui:view"` or `"kiln:invoke"`.
    pub action: String,
    /// The Cedar resource the action targets, e.g. `"a2ui:components"`.
    pub resource: String,
    /// Additional evaluation context passed to the Cedar authorizer.
    /// Currently always [`forge_domain::Json::Null`] (schemaless mode); the
    /// field exists so future Cedar context attributes don't require an API
    /// change.
    pub context: forge_domain::Json,
}

/// A policy enforcement point: evaluates a [`Request`] on behalf of an
/// authenticated subject and returns an allow/deny [`Decision`].
///
/// Implemented by [`CedarPolicyEngine`] for production use, and blanket-used
/// by [`crate::a2ui::A2UIPep`] / [`crate::kiln::KilnPep`] to derive
/// subsystem-specific capability checks.
#[async_trait]
pub trait Pep: Send + Sync {
    /// Evaluate `req` on behalf of `who` and return the resulting
    /// [`Decision`]. Implementations MUST fail closed: any internal error
    /// (parse failure, malformed identifiers, evaluator error) must produce
    /// [`Decision::Deny`], never [`Decision::Allow`].
    async fn check(&self, who: &RlsContext, req: &Request) -> Decision;
}
