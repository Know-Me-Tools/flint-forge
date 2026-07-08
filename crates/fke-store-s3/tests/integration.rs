//! Live S3 / MinIO integration tests.
//!
//! Gated behind `--features integration`; requires a running Docker daemon.
//!
//! Run (serial to avoid env-var races):
//! ```bash
//! cargo test -p fke-store-s3 --features integration -- --test-threads=1
//! ```
//!
//! Uses testcontainers v0.23 (`AsyncRunner` trait pattern — no `Cli`).
//! The `MinIO` module spins up a MinIO container on port 9000.
//!
//! NOTE: MinIO does not create buckets automatically. This test sets
//! `KILN_S3_ALLOW_HTTP=true` to enable plain-HTTP connections and expects that
//! the bucket `test-kiln` can be created on first use. In practice you may need
//! to create the bucket with the MinIO client or mc before running this test, or
//! extend the test to call the MinIO admin API.

/// Round-trip put → exists → get against a live MinIO container.
///
/// Environment variables set inside the test:
/// - `KILN_S3_BUCKET`      — `test-kiln`
/// - `KILN_S3_ENDPOINT`    — `http://localhost:<dynamic-port>`
/// - `KILN_S3_ACCESS_KEY`  — `minioadmin`
/// - `KILN_S3_SECRET_KEY`  — `minioadmin`
/// - `KILN_S3_ALLOW_HTTP`  — `true`
///
/// Run with `-- --test-threads=1` to avoid env-var races with parallel tests.
#[cfg(feature = "integration")]
#[tokio::test]
async fn test_put_get_exists_roundtrip_live() {
    use fke_ports::ComponentStore as _;
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::minio::MinIO;

    let container = MinIO::default()
        .start()
        .await
        .expect("failed to start MinIO container");

    let port = container
        .get_host_port_ipv4(9000)
        .await
        .expect("MinIO port 9000 not exposed");

    // Set env vars for StoreS3::from_env().
    // Run with -- --test-threads=1 to avoid races when tests modify env vars.
    std::env::set_var("KILN_S3_BUCKET", "test-kiln");
    std::env::set_var("KILN_S3_ENDPOINT", format!("http://localhost:{port}"));
    std::env::set_var("KILN_S3_ACCESS_KEY", "minioadmin");
    std::env::set_var("KILN_S3_SECRET_KEY", "minioadmin");
    std::env::set_var("KILN_S3_ALLOW_HTTP", "true");

    let store = fke_store_s3::StoreS3::from_env().expect("from_env failed");

    let data = b"wasm-s3-artifact";

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
}
