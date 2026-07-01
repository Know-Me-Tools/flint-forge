//! version — schema version query for the flint_meta extension.

use pgrx::prelude::*;

/// Returns the current schema version from `flint_meta.schema_version`.
/// Returns 0 if the table cannot be read or contains no rows.
#[pg_extern]
pub fn version() -> i64 {
    Spi::get_one::<i64>("SELECT COALESCE(MAX(version), 0) FROM flint_meta.schema_version")
        .unwrap_or_else(|_| None)
        .unwrap_or(0)
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_version_returns_positive() {
        let v = crate::version::version();
        assert!(v >= 1, "version should be at least 1 after bootstrap");
    }
}
