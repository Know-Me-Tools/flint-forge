//! `flint:host/kv` — ephemeral per-invocation key-value store.
//!
//! The WIT `kv` interface is **synchronous** (it never crosses an async host
//! boundary in v0.1.0) and **infallible** (the host cannot fail a get/set on
//! a per-invocation store). For that reason the trait methods below are
//! synchronous and return plain values, not `SkillResult`.
//!
//! `kv` is **not durable across invocations**. Use [`crate::Database`] for
//! persistent state.

/// Ephemeral per-invocation KV store for Flint skills.
///
/// Implement this trait as a thin adapter over the WIT-generated
/// `bindings::flint::host::kv` module.
pub trait Kv {
    /// Look up `key` in the per-invocation store, returning a copy of the
    /// stored bytes if present. Returns `None` when the key was not set in
    /// this invocation.
    fn get(&self, key: &str) -> Option<Vec<u8>>;

    /// Store `value` under `key` for the remainder of this invocation.
    /// Overwrites any prior value for the same key.
    fn set(&self, key: &str, value: &[u8]);
}
