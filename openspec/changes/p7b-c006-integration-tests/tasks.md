# p7b-c006 Tasks — Integration Test Harness

## Tasks

- [ ] Add `testcontainers = "0.23"` and `testcontainers-modules = "0.11"` to `[workspace.dependencies]`
- [ ] Add `[features] integration = []` to `fke-store-oci/Cargo.toml`
- [ ] Add `[features] integration = []` to `fke-store-ipfs/Cargo.toml`
- [ ] Add `[features] integration = []` to `fke-store-s3/Cargo.toml`
- [ ] Add `testcontainers` and `testcontainers-modules` to `[dev-dependencies]` in each store crate
- [ ] Write `#[cfg(feature = "integration")] async fn test_put_get_roundtrip_live_registry()` in `fke-store-oci/src/lib.rs` using `registry:2` container
- [ ] Write `#[cfg(feature = "integration")] async fn test_put_get_roundtrip_live_kubo()` in `fke-store-ipfs/src/lib.rs` using `ipfs/kubo` container
- [ ] Write `#[cfg(feature = "integration")] async fn test_put_get_roundtrip_live_s3()` in `fke-store-s3/src/lib.rs` using MinIO container
- [ ] Update ignored live tests to `#[cfg(feature = "integration")]` (remove `#[ignore]`)
- [ ] Verify: `cargo test -p fke-store-oci` (no features) still passes 4 unit tests
- [ ] Gate: `cargo test -p fke-store-oci --features integration` passes with Docker running
- [ ] `cargo clippy --workspace -- -D warnings` clean (feature-gated code must not introduce warnings)
