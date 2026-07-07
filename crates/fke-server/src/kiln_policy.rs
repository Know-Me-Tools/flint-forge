//! Kiln Cedar policy source — test-only allow-all bootstrap.
//!
//! `TestAllowAllPolicySource` is **test-only** (gated `#[cfg(test)]`).
//! Production uses `DbKilnPolicySource` which loads from
//! `flint_kiln.cedar_policies` and starts deny-all until the first
//! successful DB read (p7b-c002).
#![forbid(unsafe_code)]

#[cfg(test)]
use async_trait::async_trait;
#[cfg(test)]
use forge_policy::{PolicyEntry, PolicyLoadError, PolicySource};

/// Permissive policy source — allows every `kiln:invoke` action.
///
/// **For unit tests only.** Production code uses `DbKilnPolicySource`.
/// Gated `#[cfg(test)]` so this struct cannot appear in production binaries.
#[cfg(test)]
#[allow(dead_code)]
pub struct TestAllowAllPolicySource;

#[cfg(test)]
#[async_trait]
impl PolicySource for TestAllowAllPolicySource {
    async fn load(&self) -> Result<Vec<PolicyEntry>, PolicyLoadError> {
        Ok(vec![PolicyEntry {
            id: "kiln-allow-all".into(),
            text: "permit(principal, action, resource);".into(),
            enabled: true,
        }])
    }
}
