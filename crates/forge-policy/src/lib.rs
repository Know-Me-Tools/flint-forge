//! Cedar policy enforcement point (PEP) shared by Quarry mutations, the Kiln linker, and Ember.
#![forbid(unsafe_code)]

pub mod cedar;

pub use cedar::{CedarPolicyEngine, PolicyEntry, PolicyLoadError, PolicySource};

use async_trait::async_trait;
use forge_identity::RlsContext;

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
