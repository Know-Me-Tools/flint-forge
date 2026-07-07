//! Cedar policy enforcement point (PEP) shared by Quarry mutations, the Kiln linker, and Ember.
#![forbid(unsafe_code)]

pub mod a2ui;
pub mod cedar;
pub mod kiln;

pub use a2ui::{request, A2UIPep, A2UI_EMIT, A2UI_REGISTER, A2UI_RESOURCE, A2UI_VIEW};
pub use cedar::{CedarPolicyEngine, PolicyEntry, PolicyLoadError, PolicySource};
pub use kiln::{KilnPep, KILN_INVOKE, KILN_REGISTER, KILN_RESOURCE};

use async_trait::async_trait;
use forge_identity::RlsContext;

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
}

/// A capability or action request evaluated against Cedar policy.
#[derive(Debug, Clone)]
pub struct Request {
    pub action: String,
    pub resource: String,
    pub context: forge_domain::Json,
}

#[async_trait]
pub trait Pep: Send + Sync {
    async fn check(&self, who: &RlsContext, req: &Request) -> Decision;
}
