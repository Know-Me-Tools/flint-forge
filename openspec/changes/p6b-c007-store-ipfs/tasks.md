# p6b-c007 Tasks — IPFS Store

## Tasks

- [ ] Add `reqwest`, `tokio`, `anyhow` to `fke-store-ipfs/Cargo.toml`
- [ ] Add `StoreIpfs { client: reqwest::Client, base_url: String }` struct; read `FLINT_IPFS_URL` env var
- [ ] Implement `put()`: POST to `/api/v0/add`, parse `{ "Hash": "..." }` → `ContentId`
- [ ] Implement `get()`: POST to `/api/v0/cat?arg=<cid>` → bytes
- [ ] Implement `exists()`: POST to `/api/v0/stat?arg=<cid>`; 500/error → `Ok(false)`
- [ ] Unit test: mock Kubo via `wiremock` → put + get + exists round-trip
- [ ] Unit test: Kubo unavailable → `StoreError::Io`
- [ ] `cargo clippy -p fke-store-ipfs -- -D warnings` clean
