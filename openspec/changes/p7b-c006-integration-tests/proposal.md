# p7b-c006 — Integration Test Harness (OCI / IPFS / S3)

**Phase:** 7b — Kiln Production Hardening
**Priority:** P2
**Depends on:** none
**Blocks:** nothing

## What this change delivers

A `--features integration` test harness for `fke-store-oci`, `fke-store-ipfs`,
and `fke-store-s3` that spins up real infrastructure via `testcontainers` and
runs full put/get/exists round-trips against live containers.

## Design

### Feature gate

Each store crate gets:
```toml
[features]
integration = []
```

Tests tagged `#[cfg(feature = "integration")]` only run when the feature is
explicitly enabled. CI job runs them with a Docker daemon available.

### Container images

| Store | Image | Port |
|---|---|---|
| OCI | `registry:2` (Docker Hub official) | `5000` |
| IPFS | `ipfs/kubo:latest` | `5001` |
| S3 | `minio/minio:latest` | `9000` |

### `testcontainers` setup

```rust
use testcontainers::{clients::Cli, images::generic::GenericImage, Container};

#[tokio::test]
#[cfg(feature = "integration")]
async fn test_put_get_roundtrip_live_registry() {
    let docker = Cli::default();
    let registry = docker.run(GenericImage::new("registry", "2").with_exposed_port(5000));
    let port = registry.get_host_port_ipv4(5000);
    let store = StoreOci::with_registry(format!("localhost:{port}"), "test/kiln");
    let id = store.put(b"wasm bytes").await.expect("put");
    let bytes = store.get(&id).await.expect("get");
    assert_eq!(bytes, b"wasm bytes");
}
```

### Workspace deps

```toml
# [workspace.dependencies]
testcontainers = "0.23"
testcontainers-modules = "0.11"
```

Added to `[dev-dependencies]` in each store crate that uses them.

## Gate

`cargo test -p fke-store-oci --features integration` must pass against a
running Docker daemon. All three stores must pass round-trip tests.
