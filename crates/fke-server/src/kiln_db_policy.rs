//! `DbKilnPolicySource` — loads Cedar policy bundles from `flint_kiln.cedar_policies`
//! via the privileged pool.
//!
//! Direct port of `fdb-gateway/src/policy_source.rs`, pointing at the Kiln
//! policy table instead of the Quarry's `flint_meta.cedar_policies`.
//!
//! SECURITY: this MUST use the privileged pool, never a per-user RLS pool.
//! The policy table is restricted to the service role and must never be
//! accessible via the user data path.
#![forbid(unsafe_code)]

use async_trait::async_trait;
use forge_policy::{PolicyEntry, PolicyLoadError, PolicySource};
use sqlx::{PgPool, Row};

/// Loads enabled Cedar policies from `flint_kiln.cedar_policies`.
pub struct DbKilnPolicySource {
    pool: PgPool,
}

impl DbKilnPolicySource {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PolicySource for DbKilnPolicySource {
    async fn load(&self) -> Result<Vec<PolicyEntry>, PolicyLoadError> {
        let rows = sqlx::query(
            "SELECT id, policy_text, enabled \
             FROM flint_kiln.cedar_policies \
             WHERE enabled = true",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| PolicyLoadError::SourceUnavailable)?;

        let entries = rows
            .into_iter()
            .map(|r| PolicyEntry {
                id: r.get::<String, _>("id"),
                text: r.get::<String, _>("policy_text"),
                enabled: r.get::<bool, _>("enabled"),
            })
            .collect();
        Ok(entries)
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// `DbKilnPolicySource::load()` returns `SourceUnavailable` on a
    /// disconnected pool — verifying the error path without needing a DB.
    #[tokio::test]
    async fn load_returns_source_unavailable_when_pool_disconnected() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/nonexistent_kiln_test")
            .expect("lazy pool");
        let source = DbKilnPolicySource::new(pool);
        let result = source.load().await;
        assert!(
            matches!(result, Err(PolicyLoadError::SourceUnavailable)),
            "expected SourceUnavailable on disconnected pool, got {result:?}"
        );
    }
}
