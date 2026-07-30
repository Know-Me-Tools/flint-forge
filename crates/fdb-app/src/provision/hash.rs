//! Canonical spec hashing (FFS-001 §8 task 1.6).
//!
//! The hash is computed over *our* serialization of the parsed spec — struct
//! field order is fixed by declaration — so reordering fields in the input
//! JSON cannot change it, while any semantic change (a column, a type, an
//! index, table order) does. Table/column order is deliberately semantic:
//! it is the column order of the generated `CREATE TABLE`.

use fdb_domain::provision::{PlanHash, SchemaSpec};
use sha2::{Digest, Sha256};

use super::ddl::PlanError;

/// Compute the canonical `sha256:<hex>` hash of a spec.
///
/// # Errors
///
/// Returns [`PlanError::Canonicalize`] if the spec cannot be serialized —
/// structurally impossible for these types, but surfaced rather than
/// panicked on (no `expect` in library crates).
pub fn plan_hash(spec: &SchemaSpec) -> Result<PlanHash, PlanError> {
    let canonical =
        serde_json::to_vec(spec).map_err(|e| PlanError::Canonicalize(e.to_string()))?;
    let digest = Sha256::digest(&canonical);
    let mut hex = String::with_capacity(7 + digest.len() * 2);
    hex.push_str("sha256:");
    for byte in digest {
        use std::fmt::Write;
        // Infallible for String, but never unwrap in a lib crate.
        let _ = write!(hex, "{byte:02x}");
    }
    Ok(PlanHash(hex))
}
