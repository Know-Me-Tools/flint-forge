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
