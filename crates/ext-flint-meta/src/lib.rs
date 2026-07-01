//! ext-flint-meta — Flint Forge schema-cache layer (Anvil suite).
//! ----------------------------------------------------------------------------
//! Pre-computed catalog tables for the flint-reflection engine. These tables
//! are populated by DDL event triggers (p1-c008) and queried by the Rust
//! reflection engine (Phase 2). Application code must never write to them
//! directly — they are internal infrastructure.

use pgrx::prelude::*;

pg_module_magic!();

extension_sql_file!("../sql/flint_meta.sql", bootstrap);

mod agui;
mod functions;
mod keto;
mod schema;
mod triggers;
mod vault_meta;
mod version;

/// Returns the current version string of the ext-flint-meta extension.
#[pg_extern]
fn flint_meta_version() -> &'static str {
    "0.1.0"
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_flint_meta_version() {
        let result = crate::flint_meta_version();
        assert_eq!(result, "0.1.0");
    }
}

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {}
    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}
