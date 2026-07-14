//! Capability request/authorization gate for `flint:host` capabilities.

use anyhow::{bail, Result};
use fke_domain::Capability;

/// Build the Cedar request for a single `flint:host` capability. Distinct
/// from `kiln:invoke` (which gates whether the caller may invoke the
/// function at all) — this gates whether the caller may use a specific
/// governed host capability the function's manifest declares.
pub(crate) fn capability_request(cap: &Capability) -> forge_policy::Request {
    forge_policy::kiln::request(&format!(
        "kiln:capability:{}",
        capability_action_name(cap)
    ))
}

fn capability_action_name(cap: &Capability) -> &'static str {
    match cap {
        Capability::Db => "db",
        Capability::Llm => "llm",
        Capability::Kv => "kv",
        Capability::Identity => "identity",
        Capability::Secrets => "secrets",
        Capability::HttpOutgoing => "http-outgoing",
        // `Capability` is `#[non_exhaustive]` — treat any future variant as
        // deny-by-default (an unrecognized action string matches no Cedar
        // policy, so `capability_request` for it always denies) rather than
        // silently granting it.
        _ => "unknown",
    }
}

/// Verify that every capability in `required` is present in `granted`.
///
/// This is the final fail-closed gate applied after Cedar has computed which
/// of a component's manifest-declared capabilities the caller is actually
/// authorized to use (see `EdgeRuntime::granted_capabilities`): even if Cedar
/// is misconfigured or skipped (no `Pep` attached), a component can never run
/// with more capabilities than it declared as `required`, and can never
/// silently proceed if any declared capability was not granted.
///
/// # Errors
///
/// Returns an error naming the specific missing [`Capability`] as soon as one
/// required capability is not found in `granted`. `required` capabilities are
/// checked in order, so the error identifies the first missing one.
pub fn check_capabilities(required: &[Capability], granted: &[Capability]) -> Result<()> {
    for cap in required {
        if !granted.contains(cap) {
            bail!("capability {cap:?} required but not granted");
        }
    }
    Ok(())
}
