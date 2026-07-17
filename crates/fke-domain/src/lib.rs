//! Flint Kiln domain types.
//!
//! Pure, infrastructure-free types shared across the Kiln WASM component
//! edge-function gateway: content addressing (`ContentId`), the WIT-level
//! capability grants a component can request (`Capability`), the wasmtime
//! backend used to turn a component into runnable code
//! (`CompilationStrategy`), the fleet architectures Kiln can precompile for
//! (`TargetArch`), and the signed component registration record
//! (`FunctionManifest`). See spec §5.2 and §7 (`docs/FLINT-FORGE-SPEC.md`).
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use serde::{Deserialize, Serialize};

/// Content address of a component artifact (sha256 digest or IPFS CID).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ContentId(pub String);

/// A single sandboxed capability a WASM component may declare in its
/// [`FunctionManifest`] and be granted at invocation time.
///
/// The runtime host (`fke-runtime`) only wires up the WASI/WIT host
/// functions for capabilities present in the *granted* set (declared ∩
/// Cedar policy for the publisher) — anything not listed here is simply
/// unavailable to the component, regardless of what the WASM binary itself
/// imports. `#[non_exhaustive]` because new capability classes will be added
/// as Kiln grows new host integrations.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Capability {
    /// Access to the tenant's Postgres database via the Quarry ports.
    Db,
    /// Access to Flint Ember (in-DB LLM/embeddings) via the liter-llm gateway.
    Llm,
    /// Access to a key-value store scoped to the component.
    Kv,
    /// Access to identity primitives (`auth.uid()`, `auth.role()`, etc.).
    Identity,
    /// Access to Flint Vault-managed secrets.
    Secrets,
    /// Ability to make outbound HTTP requests from within the sandbox.
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

/// Which wasmtime backend compiles (or interprets) a component before it can
/// run, per spec §5.2.
///
/// This is a first-class config dimension rather than a wasmtime
/// implementation detail because Kiln's control plane and data plane are
/// built with different compiler features enabled: the control plane
/// (admin/registration server) always has Cranelift available and uses it to
/// precompile a `.cwasm` per `(digest, target)` at registration time; the
/// data plane (invocation server) has Cranelift and Winch **disabled** and
/// can only deserialize and run those pre-compiled artifacts, so the request
/// path handling untrusted, webhook-triggered input never contains a
/// compiler. `#[non_exhaustive]` so additional wasmtime backends can be
/// added without a breaking change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CompilationStrategy {
    /// Default production strategy: ahead-of-time compile to native machine
    /// code via Cranelift at registration time, cached as a `.cwasm` keyed by
    /// `(source_digest, target_arch, wasmtime_version)`. Invocation just
    /// deserializes native code, so there is zero compile latency on the hot
    /// path.
    CraneliftAot,
    /// Development-only baseline compiler: ~15-20x faster to compile than
    /// Cranelift at the cost of ~1.1-1.5x slower generated code. Used for
    /// fast local iteration; there is no Winch-to-Cranelift auto-tiering.
    Winch,
    /// Fallback portable interpreter used where Cranelift has no backend for
    /// the target architecture. Slowest of the three, but works anywhere.
    Pulley,
}

/// A fleet architecture that Flint Kiln's control plane can cross-compile a
/// registered component for.
///
/// The control plane AOT-compiles a component once per `(digest, target)`
/// pair (via its `all-arch` cross-compilation feature) so that invocation
/// servers running on any of these architectures can deserialize a matching
/// `.cwasm` without ever needing a compiler themselves. `#[non_exhaustive]`
/// because supporting additional architectures (e.g. `s390x`, `riscv64`) is
/// expected as the fleet grows.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetArch {
    /// 64-bit x86 on Linux.
    X86_64Linux,
    /// 64-bit ARM on Linux.
    Aarch64Linux,
    /// 64-bit ARM on macOS (developer machines).
    Aarch64Darwin,
}

/// Signed manifest bound to a publisher DID; granted caps = declared ∩ Cedar(publisher).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionManifest {
    /// Decentralized identifier of the publisher that signed this component,
    /// e.g. a `did:prometheus:` or `did:key:` value. Used to look up the
    /// publisher's Cedar-granted capability set and to select the verifier
    /// (`fke-sign-did` vs `fke-sign-cosign`) at registration/invocation time.
    pub publisher_did: String,
    /// Content digest of the compiled WASM artifact this manifest describes
    /// (matches [`ContentId`] once prefixed), used to fetch the bytes from a
    /// `ComponentStore` and to key the AOT compilation cache.
    pub content_digest: String,
    /// Capabilities this component *declares* it needs. The capabilities
    /// actually granted at invocation are the intersection of this list with
    /// what Cedar policy allows for `publisher_did`.
    pub capabilities: Vec<Capability>,
    /// Semantic version of this registration, unique together with the
    /// function name in `flint_kiln.functions`.
    pub version: String,
    /// RFC 3339 timestamp before which this manifest must not be treated as
    /// valid (start of the signature's validity window).
    pub not_before: String,
    /// RFC 3339 timestamp after which this manifest must no longer be
    /// treated as valid (end of the signature's validity window); expired
    /// manifests are rejected at invocation.
    pub not_after: String,
    /// Base64-encoded raw signature bytes, present when `publisher_did` uses
    /// the `did:prometheus:` scheme (verified by `fke-sign-did`'s
    /// `VerifierDid`, which needs the signature blob explicitly). Cosign-signed
    /// components (`fke-sign-cosign`'s `VerifierCosign`) look up their
    /// signature from the Rekor transparency log keyed by `content_digest` and
    /// ignore this field — it stays `None` for them. `None` for either scheme
    /// means unsigned and must be rejected at register and invoke.
    #[serde(default)]
    pub signature_b64: Option<String>,
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
