//! ComponentStore adapter: S3 / R2 / MinIO — content-addressed storage.
//!
//! Configuration is driven entirely by environment variables so the binary
//! stays secret-free at compile time.
#![forbid(unsafe_code)]

use std::sync::Arc;

use async_trait::async_trait;
use fke_domain::ContentId;
use fke_ports::{ComponentStore, StoreError};
// object_store 0.14: put/get/head moved to ObjectStoreExt (not Arc<dyn>-safe).
// Use put_opts/get_opts with GetOptions{head:true} on ObjectStore (required, dyn-safe).
use object_store::{path::Path, GetOptions, ObjectStore, PutOptions, PutPayload};
use sha2::{Digest, Sha256};

// ── Public struct ────────────────────────────────────────────────────────────

/// S3-compatible content-addressed component store.
///
/// Construct via [`StoreS3::from_env`] for production or
/// [`StoreS3::with_store`] for testing.
pub struct StoreS3 {
    store: Arc<dyn ObjectStore>,
}

// ── Construction ─────────────────────────────────────────────────────────────

impl StoreS3 {
    /// Build from environment variables.
    ///
    /// Required:
    /// - `KILN_S3_BUCKET`
    ///
    /// Optional:
    /// - `KILN_S3_ENDPOINT`   — R2/MinIO endpoint URL
    /// - `KILN_S3_ACCESS_KEY`
    /// - `KILN_S3_SECRET_KEY`
    /// - `KILN_S3_REGION`     — defaults to `us-east-1`
    pub fn from_env() -> Result<Self, StoreError> {
        use object_store::aws::AmazonS3Builder;

        let bucket = std::env::var("KILN_S3_BUCKET")
            .map_err(|_| StoreError::Io("KILN_S3_BUCKET not set".to_string()))?;

        let region = std::env::var("KILN_S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());

        let mut builder = AmazonS3Builder::new()
            .with_bucket_name(bucket)
            .with_region(region);

        if let Ok(endpoint) = std::env::var("KILN_S3_ENDPOINT") {
            builder = builder.with_endpoint(endpoint);
        }
        if let Ok(key) = std::env::var("KILN_S3_ACCESS_KEY") {
            builder = builder.with_access_key_id(key);
        }
        if let Ok(secret) = std::env::var("KILN_S3_SECRET_KEY") {
            builder = builder.with_secret_access_key(secret);
        }
        // Allow plain HTTP endpoints (e.g. local MinIO in integration tests).
        // Set KILN_S3_ALLOW_HTTP=true or KILN_S3_ALLOW_HTTP=1 to enable.
        if std::env::var("KILN_S3_ALLOW_HTTP").is_ok_and(|v| v == "true" || v == "1") {
            builder = builder.with_allow_http(true);
        }

        let store = builder.build().map_err(|e| StoreError::Io(e.to_string()))?;

        Ok(Self {
            store: Arc::new(store),
        })
    }

    /// Inject a custom [`ObjectStore`] (e.g. `InMemory`) for unit tests.
    pub fn with_store(store: Arc<dyn ObjectStore>) -> Self {
        Self { store }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// SHA-256 hex digest of `bytes`.
fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

/// Strip the `sha256:` prefix from a [`ContentId`] and return the bare hex
/// string, or an error if the prefix is absent / the id is otherwise invalid.
fn digest_from_id(id: &ContentId) -> Result<String, StoreError> {
    id.0.strip_prefix("sha256:")
        .map(ToString::to_string)
        .ok_or_else(|| StoreError::Io(format!("invalid ContentId: {}", id.0)))
}

/// Map an [`object_store::Error`] to a [`StoreError`].
fn map_store_err(e: object_store::Error) -> StoreError {
    match e {
        object_store::Error::NotFound { .. } => StoreError::NotFound,
        other => StoreError::Io(other.to_string()),
    }
}

// ── ComponentStore impl ───────────────────────────────────────────────────────

#[async_trait]
impl ComponentStore for StoreS3 {
    /// Content-address `bytes` under `sha256:<hex>` and upload to the store.
    ///
    /// The operation is idempotent: uploading the same bytes twice yields the
    /// same [`ContentId`] and simply overwrites the object (which is a no-op
    /// in practice because the content is identical).
    async fn put(&self, bytes: &[u8]) -> Result<ContentId, StoreError> {
        let hex = sha256_hex(bytes);
        let path = Path::from(hex.as_str());
        let payload = PutPayload::from(bytes.to_vec());
        self.store
            .put_opts(&path, payload, PutOptions::default())
            .await
            .map_err(map_store_err)?;
        Ok(ContentId(format!("sha256:{hex}")))
    }

    /// Retrieve the bytes stored under `id`.
    ///
    /// Returns [`StoreError::NotFound`] when no object exists for the given id.
    async fn get(&self, id: &ContentId) -> Result<Vec<u8>, StoreError> {
        let hex = digest_from_id(id)?;
        let path = Path::from(hex.as_str());
        let result = self
            .store
            .get_opts(&path, GetOptions::default())
            .await
            .map_err(map_store_err)?;
        let bytes = result.bytes().await.map_err(map_store_err)?;
        Ok(bytes.to_vec())
    }

    /// Return `true` if an object for `id` exists in the store.
    async fn exists(&self, id: &ContentId) -> Result<bool, StoreError> {
        let hex = digest_from_id(id)?;
        let path = Path::from(hex.as_str());
        // GetOptions { head: true } issues an HTTP HEAD — no body transferred.
        let opts = GetOptions {
            head: true,
            ..GetOptions::default()
        };
        match self.store.get_opts(&path, opts).await {
            Ok(_) => Ok(true),
            Err(object_store::Error::NotFound { .. }) => Ok(false),
            Err(e) => Err(StoreError::Io(e.to_string())),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use object_store::memory::InMemory;

    fn in_memory_store() -> StoreS3 {
        StoreS3::with_store(Arc::new(InMemory::new()))
    }

    #[tokio::test]
    async fn put_get_roundtrip() {
        let store = in_memory_store();
        let data = b"hello kiln";
        let id = store.put(data).await.expect("put failed");
        let retrieved = store.get(&id).await.expect("get failed");
        assert_eq!(retrieved, data);
    }

    #[tokio::test]
    async fn exists_true_after_put() {
        let store = in_memory_store();
        let id = store.put(b"some bytes").await.expect("put failed");
        assert!(store.exists(&id).await.expect("exists failed"));
    }

    #[tokio::test]
    async fn exists_false_for_missing() {
        let store = in_memory_store();
        let ghost = ContentId(
            "sha256:deadbeef00000000000000000000000000000000000000000000000000000000".to_string(),
        );
        assert!(!store.exists(&ghost).await.expect("exists failed"));
    }

    #[tokio::test]
    async fn get_missing_returns_not_found() {
        let store = in_memory_store();
        let ghost = ContentId(
            "sha256:deadbeef00000000000000000000000000000000000000000000000000000000".to_string(),
        );
        let err = store.get(&ghost).await.unwrap_err();
        assert!(matches!(err, StoreError::NotFound));
    }

    #[tokio::test]
    async fn put_idempotent_same_id() {
        let store = in_memory_store();
        let data = b"idempotent test";
        let id1 = store.put(data).await.expect("first put failed");
        let id2 = store.put(data).await.expect("second put failed");
        assert_eq!(id1, id2);
    }
}
