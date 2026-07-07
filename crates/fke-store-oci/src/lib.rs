//! ComponentStore adapter: OCI registry. Content-addressed WASM layer storage.
//!
//! Each artifact is pushed as a single-layer OCI image.  The image is tagged
//! with its sha256 content digest (colon replaced by hyphen, since OCI tags
//! forbid `:`) so that every tag is stable and idempotent.
//!
//! # Environment variables
//! | Variable            | Required | Description                              |
//! |---------------------|----------|------------------------------------------|
//! | `KILN_OCI_REGISTRY` | yes      | Registry host, e.g. `registry.example.com` |
//! | `KILN_OCI_REPO`     | yes      | Repository path, e.g. `flint/kiln`       |
//! | `KILN_OCI_USER`     | no       | Basic-auth username                      |
//! | `KILN_OCI_TOKEN`    | no       | Basic-auth password / token              |
#![forbid(unsafe_code)]

use async_trait::async_trait;
use fke_domain::ContentId;
use fke_ports::{ComponentStore, StoreError};
use oci_client::{
    client::{ClientConfig, Config, ImageLayer},
    errors::OciDistributionError,
    manifest::IMAGE_CONFIG_MEDIA_TYPE,
    secrets::RegistryAuth,
    Client, Reference,
};
use sha2::{Digest as _, Sha256};
use std::env;

/// Media type used for raw WASM/binary layer blobs.
const OCTET_STREAM: &str = "application/octet-stream";

// ────────────────────────────────────────────────────────────────────────────
// Struct
// ────────────────────────────────────────────────────────────────────────────

/// OCI registry adapter that stores WASM artifacts as content-addressed layers.
pub struct StoreOci {
    client: Client,
    /// `"<registry>/<repo>"` prefix used when building OCI `Reference`s.
    reference_prefix: String,
    auth: RegistryAuth,
}

// ────────────────────────────────────────────────────────────────────────────
// Construction
// ────────────────────────────────────────────────────────────────────────────

impl StoreOci {
    /// Build a `StoreOci` from environment variables.
    ///
    /// Returns `Err(StoreError::Io)` when either required variable is absent;
    /// never panics.
    pub fn new() -> Result<Self, StoreError> {
        let registry = env::var("KILN_OCI_REGISTRY")
            .map_err(|_| StoreError::Io("KILN_OCI_REGISTRY env var not set".to_string()))?;
        let repo = env::var("KILN_OCI_REPO")
            .map_err(|_| StoreError::Io("KILN_OCI_REPO env var not set".to_string()))?;

        let auth = match (env::var("KILN_OCI_USER"), env::var("KILN_OCI_TOKEN")) {
            (Ok(user), Ok(token)) => RegistryAuth::Basic(user, token),
            _ => RegistryAuth::Anonymous,
        };

        let client = Client::new(ClientConfig::default());

        Ok(Self {
            client,
            reference_prefix: format!("{registry}/{repo}"),
            auth,
        })
    }

    /// Build a `StoreOci` from explicit registry + repo strings (for testing).
    pub fn with_registry(registry: impl Into<String>, repo: impl Into<String>) -> Self {
        let reference_prefix = format!("{}/{}", registry.into(), repo.into());
        Self {
            client: Client::new(ClientConfig::default()),
            reference_prefix,
            auth: RegistryAuth::Anonymous,
        }
    }

    /// Build a `StoreOci` using plain HTTP — for local registries and
    /// testcontainers-backed integration tests where TLS is not available.
    pub fn with_http_registry(registry: impl Into<String>, repo: impl Into<String>) -> Self {
        let reference_prefix = format!("{}/{}", registry.into(), repo.into());
        Self {
            client: Client::new(ClientConfig {
                protocol: oci_client::client::ClientProtocol::Http,
                ..ClientConfig::default()
            }),
            reference_prefix,
            auth: RegistryAuth::Anonymous,
        }
    }

    /// Convert a `ContentId` like `"sha256:<hex>"` to an OCI `Reference`.
    ///
    /// OCI tags forbid `:`, so the colon is replaced with a hyphen:
    /// `sha256:abcd…` → tag `sha256-abcd…`.
    fn make_reference(&self, id: &ContentId) -> Result<Reference, StoreError> {
        let tag = id.0.replace(':', "-");
        Reference::try_from(format!("{}:{}", self.reference_prefix, tag))
            .map_err(|e| StoreError::Io(format!("invalid OCI reference: {e}")))
    }

    /// Map an `OciDistributionError` to the appropriate `StoreError` variant.
    fn map_oci_err(e: OciDistributionError) -> StoreError {
        match e {
            OciDistributionError::ImageManifestNotFoundError(_)
            | OciDistributionError::ServerError { code: 404, .. } => StoreError::NotFound,
            other => StoreError::Io(other.to_string()),
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// ComponentStore impl
// ────────────────────────────────────────────────────────────────────────────

#[async_trait]
impl ComponentStore for StoreOci {
    /// Compute the sha256 digest, push one OCI layer, and return the
    /// `ContentId`.  Idempotent: if the manifest already exists the push is
    /// skipped and the same `ContentId` is returned.
    async fn put(&self, bytes: &[u8]) -> Result<ContentId, StoreError> {
        // 1. Compute content digest.
        let hex = {
            let mut h = Sha256::new();
            h.update(bytes);
            format!("{:x}", h.finalize())
        };
        let id = ContentId(format!("sha256:{hex}"));

        // 2. Skip push when already present (idempotent).
        if self.exists(&id).await? {
            return Ok(id);
        }

        // 3. Build the OCI reference and layer payload.
        let reference = self.make_reference(&id)?;
        let layer = ImageLayer::new(bytes.to_vec(), OCTET_STREAM.to_string(), None);
        let config = Config::new(b"{}".to_vec(), IMAGE_CONFIG_MEDIA_TYPE.to_string(), None);

        // 4. Push layer + auto-generated manifest.
        self.client
            .push(&reference, &[layer], config, &self.auth, None)
            .await
            .map_err(Self::map_oci_err)?;

        Ok(id)
    }

    /// Pull the single layer stored under `id` and return its raw bytes.
    async fn get(&self, id: &ContentId) -> Result<Vec<u8>, StoreError> {
        let reference = self.make_reference(id)?;

        let data = self
            .client
            .pull(&reference, &self.auth, vec![OCTET_STREAM])
            .await
            .map_err(Self::map_oci_err)?;

        data.layers
            .into_iter()
            .next()
            .map(|l| l.data.to_vec())
            .ok_or(StoreError::NotFound)
    }

    /// Returns `true` when the manifest for `id` exists in the registry.
    async fn exists(&self, id: &ContentId) -> Result<bool, StoreError> {
        let reference = self.make_reference(id)?;

        match self
            .client
            .fetch_manifest_digest(&reference, &self.auth)
            .await
        {
            Ok(_) => Ok(true),
            Err(
                OciDistributionError::ImageManifestNotFoundError(_)
                | OciDistributionError::ServerError { code: 404, .. },
            ) => Ok(false),
            Err(e) => Err(StoreError::Io(e.to_string())),
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// `StoreOci::new()` must return `Err` (not panic) when the required env
    /// vars are absent.
    #[test]
    fn new_fails_gracefully_without_env_vars() {
        // Remove vars so the test environment is clean.
        env::remove_var("KILN_OCI_REGISTRY");
        env::remove_var("KILN_OCI_REPO");

        let result = StoreOci::new();
        assert!(
            result.is_err(),
            "expected Err when KILN_OCI_REGISTRY is missing"
        );
    }

    /// sha256 is deterministic: identical bytes always produce the same digest.
    #[test]
    fn content_id_is_deterministic_sha256() {
        // echo -n "hello" | sha256sum
        // 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
        let input = b"hello";
        let mut h = Sha256::new();
        h.update(input);
        let hex = format!("{:x}", h.finalize());
        assert_eq!(
            hex,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );

        let id = ContentId(format!("sha256:{hex}"));
        assert_eq!(
            id.0,
            "sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    /// `make_reference` must replace the colon in `sha256:` with a hyphen to
    /// produce a valid OCI tag. Uses `with_registry` to avoid env var races.
    #[test]
    fn make_reference_replaces_colon_with_hyphen() {
        let store = StoreOci::with_registry("registry.example.com", "flint/kiln");
        let id = ContentId("sha256:abcdef1234".to_string());
        let reference = store.make_reference(&id).expect("valid reference");

        let ref_str = reference.to_string();
        assert!(
            ref_str.contains("sha256-abcdef1234"),
            "expected tag with hyphen, got: {ref_str}"
        );
        assert!(
            !ref_str.contains("sha256:abcdef"),
            "colon must not appear in tag"
        );
    }

    /// Two `ContentId`s derived from the same bytes must be identical.
    #[test]
    fn put_produces_same_id_for_same_bytes() {
        // We cannot call the async `put` here without a registry, but we can
        // verify the digest computation that `put` relies on.
        let bytes = b"deterministic-test-payload";
        let digest_a = {
            let mut h = Sha256::new();
            h.update(bytes);
            format!("sha256:{:x}", h.finalize())
        };
        let digest_b = {
            let mut h = Sha256::new();
            h.update(bytes);
            format!("sha256:{:x}", h.finalize())
        };
        assert_eq!(digest_a, digest_b);
    }

    /// Live-registry round-trip: put → exists → get.
    ///
    /// Run with:
    /// ```bash
    /// KILN_OCI_REGISTRY=registry.example.com \
    /// KILN_OCI_REPO=flint/kiln \
    /// KILN_OCI_USER=user \
    /// KILN_OCI_TOKEN=token \
    /// cargo test -p fke-store-oci -- --ignored
    /// ```
    #[tokio::test]
    #[ignore = "requires a running OCI registry at KILN_OCI_REGISTRY"]
    async fn test_put_get_exists_roundtrip() {
        let store = StoreOci::new().expect("env vars must be set for live test");

        let payload = b"test-wasm-artifact-bytes";

        // put
        let id = store.put(payload).await.expect("put should succeed");
        assert!(id.0.starts_with("sha256:"), "id must carry sha256 prefix");

        // exists
        assert!(
            store.exists(&id).await.expect("exists should succeed"),
            "artifact must be present after put"
        );

        // get
        let got = store.get(&id).await.expect("get should succeed");
        assert_eq!(got, payload, "round-tripped bytes must match");

        // idempotent put
        let id2 = store.put(payload).await.expect("second put should succeed");
        assert_eq!(id, id2, "same bytes must yield same ContentId");
    }

    /// `get` on an unknown id must return `StoreError::NotFound` (not panic).
    #[tokio::test]
    #[ignore = "requires a running OCI registry at KILN_OCI_REGISTRY"]
    async fn get_unknown_returns_not_found() {
        let store = StoreOci::new().expect("env vars must be set for live test");
        let unknown = ContentId(
            "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
        );
        let err = store.get(&unknown).await.expect_err("must error");
        assert!(
            matches!(err, StoreError::NotFound),
            "expected NotFound, got: {err}"
        );
    }
}
