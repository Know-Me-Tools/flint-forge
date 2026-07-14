//! Flint Kiln domain types.
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Content address of a component artifact (sha256 digest or IPFS CID).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ContentId(pub String);

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Capability {
    Db,
    Llm,
    Kv,
    Identity,
    Secrets,
    HttpOutgoing,
}

impl Capability {
    /// Lowercase identifier used to build the Cedar action name
    /// `kiln:capability:<name>` (see `forge_policy::kiln::capability_action`).
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Capability::Db => "db",
            Capability::Llm => "llm",
            Capability::Kv => "kv",
            Capability::Identity => "identity",
            Capability::Secrets => "secrets",
            Capability::HttpOutgoing => "http_outgoing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CompilationStrategy {
    CraneliftAot,
    Winch,
    Pulley,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetArch {
    X86_64Linux,
    Aarch64Linux,
    Aarch64Darwin,
}

/// Signed manifest bound to a publisher DID; granted caps = declared ∩ Cedar(publisher).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionManifest {
    pub publisher_did: String,
    pub content_digest: String,
    pub capabilities: Vec<Capability>,
    pub version: String,
    pub not_before: String,
    pub not_after: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_as_str_is_distinct_per_variant() {
        let names: Vec<&str> = [
            Capability::Db,
            Capability::Llm,
            Capability::Kv,
            Capability::Identity,
            Capability::Secrets,
            Capability::HttpOutgoing,
        ]
        .iter()
        .map(Capability::as_str)
        .collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len(), "capability names must be unique");
    }
}
