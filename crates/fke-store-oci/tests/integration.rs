//! Live OCI registry integration tests.
//!
//! Gated behind `--features integration`; requires a running Docker daemon.
//!
//! Run:
//! ```bash
//! cargo test -p fke-store-oci --features integration -- --test-threads=1
//! ```
//!
//! Uses testcontainers v0.23 (`AsyncRunner` trait pattern — no `Cli`).
//! The `CncfDistribution` module (`cncf_distribution` feature) spins up the
//! official CNCF distribution registry image on port 5000.

/// Round-trip put → exists → get against a live `CncfDistribution` registry
/// container managed by testcontainers.
#[cfg(feature = "integration")]
#[tokio::test]
async fn test_put_get_exists_roundtrip_live() {
    use fke_ports::ComponentStore as _;
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::cncf_distribution::CncfDistribution;

    let container = CncfDistribution::default()
        .start()
        .await
        .expect("failed to start CncfDistribution registry container");

    let port = container
        .get_host_port_ipv4(5000)
        .await
        .expect("registry port 5000 not exposed");

    // Use `with_http_registry` so the client speaks plain HTTP to localhost.
    let store =
        fke_store_oci::StoreOci::with_http_registry(format!("localhost:{port}"), "test/kiln");

    let data = b"wasm-artifact-bytes";

    let id = store.put(data).await.expect("put failed");
    assert!(
        id.0.starts_with("sha256:"),
        "ContentId must have sha256 prefix"
    );

    assert!(
        store.exists(&id).await.expect("exists failed"),
        "artifact must be present after put"
    );

    let got = store.get(&id).await.expect("get failed");
    assert_eq!(got, data, "round-tripped bytes must match");

    // Idempotent second put must return the same ContentId.
    let id2 = store.put(data).await.expect("second put failed");
    assert_eq!(id, id2, "same bytes must yield the same ContentId");
}
