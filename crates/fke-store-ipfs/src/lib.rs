//! ComponentStore adapter: IPFS (Kubo HTTP API). Content-addressed.
#![forbid(unsafe_code)]

use async_trait::async_trait;
use fke_domain::ContentId;
use fke_ports::{ComponentStore, StoreError};

const DEFAULT_IPFS_URL: &str = "http://localhost:5001";
const ENV_IPFS_URL: &str = "FLINT_IPFS_URL";
/// Multipart boundary used when uploading to `/api/v0/add`.
const MULTIPART_BOUNDARY: &str = "----FlintIpfsBoundary";

/// Kubo HTTP API adapter for content-addressed component storage.
pub struct StoreIpfs {
    client: reqwest::Client,
    base_url: String,
}

impl StoreIpfs {
    /// Create a new adapter, reading `FLINT_IPFS_URL` from the environment.
    /// Falls back to `http://localhost:5001` when the variable is not set.
    #[must_use]
    pub fn new() -> Self {
        let base_url = std::env::var(ENV_IPFS_URL)
            .unwrap_or_else(|_| DEFAULT_IPFS_URL.to_owned());
        Self::with_url(base_url)
    }

    /// Create an adapter pointing at an explicit base URL (useful in tests).
    #[must_use]
    pub fn with_url(url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: url.into(),
        }
    }
}

impl Default for StoreIpfs {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ComponentStore for StoreIpfs {
    /// Upload bytes to IPFS via `POST /api/v0/add`.
    ///
    /// Returns the CID reported by Kubo in the `Hash` field of the JSON response.
    async fn put(&self, bytes: &[u8]) -> Result<ContentId, StoreError> {
        let url = format!("{}/api/v0/add", self.base_url);

        // Build a multipart/form-data body manually — the workspace's reqwest
        // feature set does not include the `multipart` feature, so we construct
        // the wire format by hand.
        let mut body: Vec<u8> = Vec::new();
        body.extend_from_slice(format!("--{MULTIPART_BOUNDARY}\r\n").as_bytes());
        body.extend_from_slice(
            b"Content-Disposition: form-data; name=\"file\"; filename=\"component\"\r\n",
        );
        body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
        body.extend_from_slice(bytes);
        body.extend_from_slice(format!("\r\n--{MULTIPART_BOUNDARY}--\r\n").as_bytes());

        let content_type = format!("multipart/form-data; boundary={MULTIPART_BOUNDARY}");

        let resp = self
            .client
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, content_type)
            .body(body)
            .send()
            .await
            .map_err(|e| StoreError::Io(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(StoreError::Io(format!("IPFS add failed {status}: {body}")));
        }

        let body = resp.text().await.map_err(|e| StoreError::Io(e.to_string()))?;
        let value: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| StoreError::Io(e.to_string()))?;

        let hash = value
            .get("Hash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StoreError::Io("missing Hash in IPFS add response".to_owned()))?;

        Ok(ContentId(hash.to_owned()))
    }

    /// Fetch content by CID via `POST /api/v0/cat?arg=<cid>`.
    async fn get(&self, id: &ContentId) -> Result<Vec<u8>, StoreError> {
        let url = format!("{}/api/v0/cat", self.base_url);

        let resp = self
            .client
            .post(&url)
            .query(&[("arg", &id.0)])
            .send()
            .await
            .map_err(|e| StoreError::Io(e.to_string()))?;

        match resp.status() {
            s if s.as_u16() == 404 => return Err(StoreError::NotFound),
            s if !s.is_success() => {
                let body = resp.text().await.unwrap_or_default();
                return Err(StoreError::Io(format!("IPFS cat failed {s}: {body}")));
            }
            _ => {}
        }

        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| StoreError::Io(e.to_string()))
    }

    /// Check existence via `POST /api/v0/stat?arg=<cid>`.
    ///
    /// A 200 response means the CID is available. Any error (including HTTP 500
    /// or a connection failure) is treated as `Ok(false)` — the node is either
    /// unreachable or does not have the block.
    async fn exists(&self, id: &ContentId) -> Result<bool, StoreError> {
        let url = format!("{}/api/v0/stat", self.base_url);

        let Ok(resp) = self
            .client
            .post(&url)
            .query(&[("arg", &id.0)])
            .send()
            .await
        else {
            return Ok(false);
        };

        Ok(resp.status().is_success())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn put_returns_cid_from_response() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v0/add"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"Hash":"QmTest123"}"#),
            )
            .mount(&server)
            .await;

        let store = StoreIpfs::with_url(server.uri());
        let cid = store.put(b"hello wasm").await.expect("put should succeed");
        assert_eq!(cid, ContentId("QmTest123".to_owned()));
    }

    #[tokio::test]
    async fn get_returns_bytes() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v0/cat"))
            .respond_with(
                ResponseTemplate::new(200).set_body_bytes(b"wasm bytes".as_ref()),
            )
            .mount(&server)
            .await;

        let store = StoreIpfs::with_url(server.uri());
        let bytes = store
            .get(&ContentId("QmTest".to_owned()))
            .await
            .expect("get should succeed");
        assert_eq!(bytes, b"wasm bytes");
    }

    #[tokio::test]
    async fn exists_true_when_stat_200() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v0/stat"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
            .mount(&server)
            .await;

        let store = StoreIpfs::with_url(server.uri());
        let result = store
            .exists(&ContentId("QmTest".to_owned()))
            .await
            .expect("exists should not error");
        assert!(result);
    }

    #[tokio::test]
    async fn exists_false_when_stat_500() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v0/stat"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let store = StoreIpfs::with_url(server.uri());
        let result = store
            .exists(&ContentId("QmTest".to_owned()))
            .await
            .expect("exists should not error on 500");
        assert!(!result);
    }

    #[tokio::test]
    async fn get_404_returns_not_found() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/v0/cat"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let store = StoreIpfs::with_url(server.uri());
        let err = store
            .get(&ContentId("QmMissing".to_owned()))
            .await
            .expect_err("get of missing CID should fail");
        assert!(
            matches!(err, StoreError::NotFound),
            "expected NotFound, got {err:?}"
        );
    }
}
