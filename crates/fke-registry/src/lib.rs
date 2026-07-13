//! Flint Kiln function registry backed by Postgres.
//!
//! Resolves function name@version to a `FunctionManifest` and the raw WASM
//! bytes from `flint_kiln.functions`. Falls back to `StoreError::NotFound`
//! when no matching row exists.
#![forbid(unsafe_code)]

use async_trait::async_trait;
use fke_domain::{ContentId, FunctionManifest};
use fke_ports::{ComponentRegistry, ComponentStore, StoreError};
use sha2::{Digest, Sha256};
use sqlx::{types::Json as SqlxJson, FromRow, PgPool};

/// Postgres-backed component registry.
pub struct PgRegistry {
    pool: PgPool,
}

impl PgRegistry {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, FromRow)]
struct FunctionRow {
    manifest: SqlxJson<FunctionManifest>,
    #[allow(dead_code)]
    content_digest: String,
}

#[async_trait]
impl ComponentRegistry for PgRegistry {
    async fn resolve(&self, name: &str, version: &str) -> Result<FunctionManifest, StoreError> {
        let row: Option<FunctionRow> = sqlx::query_as(
            "SELECT manifest, content_digest
             FROM flint_kiln.functions
             WHERE name = $1 AND version = $2 AND active = true
             LIMIT 1",
        )
        .bind(name)
        .bind(version)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StoreError::Io(e.to_string()))?;

        row.map(|r| r.manifest.0).ok_or(StoreError::NotFound)
    }
}

/// Postgres-backed WASM artifact store.
/// Stores compressed component bytes in `flint_kiln.artifacts`.
pub struct PgComponentStore {
    pool: PgPool,
}

impl PgComponentStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Expose the pool for direct SQL in callers that need it.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl ComponentStore for PgComponentStore {
    async fn put(&self, bytes: &[u8]) -> Result<ContentId, StoreError> {
        let digest = sha256_hex(bytes);
        let id = ContentId(format!("sha256:{digest}"));
        sqlx::query(
            "INSERT INTO flint_kiln.artifacts (content_digest, bytes)
             VALUES ($1, $2)
             ON CONFLICT (content_digest) DO NOTHING",
        )
        .bind(&digest)
        .bind(bytes)
        .execute(&self.pool)
        .await
        .map_err(|e| StoreError::Io(e.to_string()))?;
        Ok(id)
    }

    async fn get(&self, id: &ContentId) -> Result<Vec<u8>, StoreError> {
        let digest = id.0.strip_prefix("sha256:").unwrap_or(&id.0);
        let row: Option<(Vec<u8>,)> =
            sqlx::query_as("SELECT bytes FROM flint_kiln.artifacts WHERE content_digest = $1")
                .bind(digest)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| StoreError::Io(e.to_string()))?;
        row.map(|(b,)| b).ok_or(StoreError::NotFound)
    }

    async fn exists(&self, id: &ContentId) -> Result<bool, StoreError> {
        let digest = id.0.strip_prefix("sha256:").unwrap_or(&id.0);
        let row: Option<(bool,)> =
            sqlx::query_as("SELECT true FROM flint_kiln.artifacts WHERE content_digest = $1")
                .bind(digest)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| StoreError::Io(e.to_string()))?;
        Ok(row.is_some())
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hex_stable_for_same_input() {
        let a = sha256_hex(b"hello");
        let b = sha256_hex(b"hello");
        assert_eq!(a, b);
    }

    #[test]
    fn sha256_hex_differs_for_different_input() {
        let a = sha256_hex(b"hello");
        let b = sha256_hex(b"world");
        assert_ne!(a, b);
    }

    /// p16-c002: this is a *real* SHA-256, not the prior pseudo-hash — assert
    /// against the actual well-known digest vectors, not just stability.
    #[test]
    fn sha256_hex_matches_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        );
    }
}
