# p6b-c008 Tasks — S3/R2 Store

## Tasks

- [ ] Add `object_store = { version = "0.11", features = ["aws"] }` to `[workspace.dependencies]`; add `object_store`, `sha2`, `tokio`, `anyhow` to `fke-store-s3/Cargo.toml`
- [ ] Build `AmazonS3Builder` from `KILN_S3_BUCKET` + optional `KILN_S3_ENDPOINT` (for R2/MinIO) in `StoreS3::new()`
- [ ] Implement `put()`: sha256 digest → `object_store.put(path, payload)` → `ContentId`
- [ ] Implement `get()`: `object_store.get(path) → GetResult → bytes`; 404 → `StoreError::NotFound`
- [ ] Implement `exists()`: `object_store.head(path).await.is_ok()`
- [ ] Unit test: put + get + exists round-trip against `object_store::memory::InMemory` store
- [ ] Unit test: missing key → `StoreError::NotFound`
- [ ] `cargo clippy -p fke-store-s3 -- -D warnings` clean
