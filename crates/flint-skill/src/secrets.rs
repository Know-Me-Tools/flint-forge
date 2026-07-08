//! `flint:host/secrets` — Cedar-gated secret access.
//!
//! The WIT `secrets` interface is split across two surfaces:
//!
//! - `get(name) -> result<secret, host-error>` — returns an opaque
//!   `secret` resource handle. The raw value is **not** in the handle.
//! - `secret.reveal() -> result<string, host-error>` — the escape hatch that
//!   returns the raw secret value. Cedar-gated, audited, default-deny.
//!
//! High-value secrets are brokered at the host boundary (the host injects
//! them into outbound calls; the secret never enters WASM linear memory).
//! Components should prefer host-brokered calls and only `reveal` when they
//! genuinely must read the value.

use crate::error::SkillResult;
use std::future::Future;

/// Opaque handle to a secret resolved by [`Secrets::get`].
///
/// The handle does **not** hold the raw value. Call [`Self::reveal`] to read
/// it, understanding that every `reveal` call is:
///
/// - **Cedar-gated** — the publisher must hold an explicit Cedar grant.
/// - **Audited** — every call is logged in `vault.access_log`.
/// - **Default-deny** — no grant → `reveal` returns an error.
///
/// Implement this trait as a thin adapter over the WIT-generated
/// `bindings::flint::host::secrets::Secret` resource.
pub trait SecretHandle {
    /// Return the raw secret value.
    ///
    /// Skill code should treat the returned `String` as confidential: never
    /// log it, never echo it into an HTTP response body, never persist it to
    /// [`crate::Kv`]. The recommended pattern is to forward it directly into
    /// an outbound call via the host's outgoing-handler so it never lives
    /// longer than necessary in WASM linear memory.
    fn reveal(&self) -> impl Future<Output = SkillResult<String>> + Send;
}

/// Cedar-gated secret resolution for Flint skills.
///
/// Implement this trait as a thin adapter over the WIT-generated
/// `bindings::flint::host::secrets` module.
pub trait Secrets {
    /// Type of the opaque handle returned by [`Self::get`]. Implements
    /// [`SecretHandle`] so skill code can `reveal` it.
    type Handle: SecretHandle + Send + Sync;

    /// Resolve `name` to an opaque secret handle.
    ///
    /// Fails with [`crate::SkillError::Secrets`] `{ code: "CEDAR_DENY" }`
    /// when the publisher holds no grant, or `{ code: "NOT_FOUND" }` when the
    /// secret does not exist in Flint Vault.
    fn get(&self, name: &str) -> impl Future<Output = SkillResult<Self::Handle>> + Send;
}
