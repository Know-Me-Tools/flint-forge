//! Live IPFS integration tests.
//!
//! Gated behind `--features integration`; requires a running Kubo (go-ipfs) node.
//!
//! Run:
//! ```bash
//! FLINT_IPFS_URL=http://localhost:5001 \
//! cargo test -p fke-store-ipfs --features integration -- --test-threads=1
//! ```
//!
//! The store reads `FLINT_IPFS_URL` from the environment (defaults to
//! `http://localhost:5001`) so you can point it at any running Kubo node.

/// Round-trip put → exists → get against a live Kubo IPFS node.
///
/// Reads the Kubo API endpoint from `FLINT_IPFS_URL`
/// (default: `http://localhost:5001`).
#[cfg(feature = "integration")]
#[tokio::test]
async fn test_put_get_exists_roundtrip_live() {
    use fke_ports::ComponentStore as _;
    let store = fke_store_ipfs::StoreIpfs::new();

    let data = b"wasm-artifact";

    let id = store.put(data).await.expect("put failed");
    assert!(!id.0.is_empty(), "CID must not be empty");

    assert!(
        store.exists(&id).await.expect("exists failed"),
        "artifact must be present after put"
    );

    let got = store.get(&id).await.expect("get failed");
    assert_eq!(got, data, "round-tripped bytes must match");
}
