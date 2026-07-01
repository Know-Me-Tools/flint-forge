//! `DbPolicySource` — loads Cedar policy bundles from `flint_meta.cedar_policies`
//! via the privileged (service_role) pool.
//!
//! This is the adapter the `forge-policy` crate documents but deliberately does
//! not implement (to keep `forge-policy` free of `sqlx`). The composition root
//! wires it into a `CedarPolicyEngine`.
//!
//! SECURITY: this MUST use the privileged pool, never the user RLS pool — the
//! policy table is `service_role`-only and never exposed under RLS.

use async_trait::async_trait;
use forge_policy::{PolicyEntry, PolicyLoadError, PolicySource};
use sqlx::{PgPool, Row};

/// Loads enabled Cedar policies from `flint_meta.cedar_policies`.
pub struct DbPolicySource {
    pool: PgPool,
}

impl DbPolicySource {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PolicySource for DbPolicySource {
    async fn load(&self) -> Result<Vec<PolicyEntry>, PolicyLoadError> {
        let rows = sqlx::query(
            "SELECT id, policy_text, enabled \
             FROM flint_meta.cedar_policies \
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
