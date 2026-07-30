//! Canonical plan hashing (FFS-001 §8 task 1.6, D2 drift guard).
//!
//! The hash covers the canonicalized spec **and** the generated DDL. The
//! spec half makes JSON field reordering irrelevant (we hash *our*
//! serialization of the parsed struct, whose field order is fixed by
//! declaration) while any semantic change — a column, a type, an index,
//! table order — changes it. The DDL half is what makes the apply-time
//! drift guard real: `generate()` is a function of `(spec, live)`, so if the
//! live schema drifted between plan and apply, re-planning the stored spec
//! yields different DDL and therefore a different hash — a spec-only hash
//! would trivially always match its own re-plan and could never detect
//! drift. Table/column order is deliberately semantic: it is the column
//! order of the generated `CREATE TABLE`.

use fdb_domain::provision::{PlanHash, SchemaSpec};
use sha2::{Digest, Sha256};

use super::ddl::PlanError;

/// Compute the canonical `sha256:<hex>` hash of a spec plus its generated
/// DDL.
///
/// # Errors
///
/// Returns [`PlanError::Canonicalize`] if the spec cannot be serialized —
/// structurally impossible for these types, but surfaced rather than
/// panicked on (no `expect` in library crates).
pub fn plan_hash(spec: &SchemaSpec, ddl: &str) -> Result<PlanHash, PlanError> {
    let canonical = serde_json::to_vec(spec).map_err(|e| PlanError::Canonicalize(e.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(&canonical);
    hasher.update([0u8]);
    hasher.update(ddl.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(7 + digest.len() * 2);
    hex.push_str("sha256:");
    for byte in digest {
        use std::fmt::Write;
        // Infallible for String, but never unwrap in a lib crate.
        let _ = write!(hex, "{byte:02x}");
    }
    Ok(PlanHash(hex))
}
