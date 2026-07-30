//! Schema-provisioning use-cases (FFS-001, p17): the pure DDL generator and
//! the canonical plan hash.
//!
//! `generate()` is deliberately a pure function of `(spec, live)` — no
//! database, no clock, no randomness — so the highest-risk code in the
//! feature is unit-testable and deterministic (same spec ⇒ same DDL ⇒ same
//! hash). Plan identity (`PlanId`) and expiry are minted by the gateway at
//! persist time, never here.

pub mod ddl;
pub mod hash;

#[cfg(test)]
mod tests;

pub use ddl::{generate, PlanError};
pub use hash::plan_hash;
