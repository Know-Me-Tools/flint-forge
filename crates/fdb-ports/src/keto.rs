//! Keto coarse relationship check — the subscribe-time and mutation-time gate.
//!
//! # Hexagonal rule
//!
//! This trait lives in `fdb-ports`. Concrete implementations live in adapter
//! crates (`fdb-gateway::keto_sync::KetoCacheAdapter`). `fdb-app` imports this
//! trait only — never the adapter.
//!
//! # Security
//!
//! - **Fail-closed:** if the check cannot be performed (cache miss, service
//!   unreachable, internal error), implementations MUST return `false`.
//!   Denying is always the safe default.
//! - **PII:** `subject` values are personally identifiable. Implementations
//!   MUST NOT log, trace, or expose `subject` at any level.

use async_trait::async_trait;

/// Coarse relationship check against Keto (Ory Permissions Service).
///
/// Returns `true` when `subject` holds `relation` on `object` within
/// `namespace`. Returns `false` on any denial or internal failure
/// (fail-closed semantics).
///
/// # Arguments
///
/// * `namespace` — the Keto namespace (e.g. `"entities"`).
/// * `object` — the object identifier within the namespace (e.g. `"orders"`).
/// * `relation` — the relation to check (e.g. `"view"`, `"edit"`).
/// * `subject` — the subject identifier (PII — never logged).
#[async_trait]
pub trait KetoCheck: Send + Sync {
    async fn check(
        &self,
        namespace: &str,
        object: &str,
        relation: &str,
        subject: &str,
    ) -> bool;
}
