# p6b-c006 Tasks — OCI Store

## Tasks

- [ ] Add `oci-client = "0.14"` to `[workspace.dependencies]`; add `oci-client`, `sha2`, `tokio`, `anyhow` to `fke-store-oci/Cargo.toml`
- [ ] Add `StoreOci { client, registry, repository }` struct; read `KILN_OCI_REGISTRY` + `KILN_OCI_REPO` env vars in `StoreOci::new()`
- [ ] Implement `put()`: compute sha256, push OCI layer + minimal manifest, return `ContentId("sha256:<hex>")`
- [ ] Implement `get()`: pull layer by digest, return raw bytes
- [ ] Implement `exists()`: HEAD manifest at digest; 404 → `Ok(false)`
- [ ] Unit test: put + exists + get round-trip against a local `testcontainers` registry OR mock HTTP
- [ ] Unit test: get missing digest → `StoreError::NotFound`
- [ ] `cargo clippy -p fke-store-oci -- -D warnings` clean
