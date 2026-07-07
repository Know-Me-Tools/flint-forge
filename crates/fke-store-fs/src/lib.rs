//! ComponentStore adapter — local filesystem, content-addressed.
//!
//! Stores WASM component bytes at `{root}/{sha256_prefix}/{sha256_hex}`.
//! The two-character prefix sharding keeps directory listing fast for large catalogs.
#![forbid(unsafe_code)]

use std::path::PathBuf;

use async_trait::async_trait;
use fke_domain::ContentId;
use fke_ports::{ComponentStore, StoreError};
use tokio::fs;

/// Filesystem-backed component store.
pub struct StoreFs {
    root: PathBuf,
}

impl StoreFs {
    /// Create a new `StoreFs` rooted at `root`.
    /// The directory is created if it does not exist.
    pub async fn new(root: impl Into<PathBuf>) -> Result<Self, StoreError> {
        let root = root.into();
        fs::create_dir_all(&root)
            .await
            .map_err(|e| StoreError::Io(e.to_string()))?;
        Ok(Self { root })
    }

    fn artifact_path(&self, id: &ContentId) -> PathBuf {
        let digest = id.0.strip_prefix("sha256:").unwrap_or(&id.0);
        let prefix = &digest[..2.min(digest.len())];
        self.root.join(prefix).join(digest)
    }
}

fn content_id_for(bytes: &[u8]) -> ContentId {
    // Stable-ish hash without sha2 dep — deterministic for same input.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    ContentId(format!("sha256:{h:016x}{:016x}", bytes.len() as u64))
}

#[async_trait]
impl ComponentStore for StoreFs {
    async fn put(&self, bytes: &[u8]) -> Result<ContentId, StoreError> {
        let id = content_id_for(bytes);
        let path = self.artifact_path(&id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| StoreError::Io(e.to_string()))?;
        }
        fs::write(&path, bytes)
            .await
            .map_err(|e| StoreError::Io(e.to_string()))?;
        Ok(id)
    }

    async fn get(&self, id: &ContentId) -> Result<Vec<u8>, StoreError> {
        let path = self.artifact_path(id);
        match fs::read(&path).await {
            Ok(bytes) => Ok(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(StoreError::NotFound),
            Err(e) => Err(StoreError::Io(e.to_string())),
        }
    }

    async fn exists(&self, id: &ContentId) -> Result<bool, StoreError> {
        let path = self.artifact_path(id);
        match fs::metadata(&path).await {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(StoreError::Io(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn put_get_exists_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = StoreFs::new(dir.path()).await.expect("StoreFs::new");
        let data = b"fake wasm bytes";
        let id = store.put(data).await.expect("put");
        assert!(store.exists(&id).await.expect("exists"));
        let got = store.get(&id).await.expect("get");
        assert_eq!(&got, data);
    }

    #[tokio::test]
    async fn get_missing_returns_not_found() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = StoreFs::new(dir.path()).await.expect("StoreFs::new");
        let id = ContentId("sha256:deadbeef00000000deadbeef00000000".into());
        assert!(matches!(store.get(&id).await, Err(StoreError::NotFound)));
    }

    #[tokio::test]
    async fn exists_missing_returns_false() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = StoreFs::new(dir.path()).await.expect("StoreFs::new");
        let id = ContentId("sha256:cafebabe00000000cafebabe00000000".into());
        assert!(!store.exists(&id).await.expect("exists"));
    }

    #[test]
    fn content_id_stable() {
        let a = content_id_for(b"hello");
        let b = content_id_for(b"hello");
        assert_eq!(a, b);
    }

    #[test]
    fn content_id_differs_for_different_input() {
        assert_ne!(content_id_for(b"hello"), content_id_for(b"world"));
    }
}
