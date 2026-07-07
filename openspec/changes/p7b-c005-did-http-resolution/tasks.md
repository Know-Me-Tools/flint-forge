# p7b-c005 Tasks — DID HTTP Resolution

## Tasks

- [ ] Add `reqwest = { workspace = true }`, `serde_json = { workspace = true }`, `tokio = { workspace = true }` to `fke-sign-did/Cargo.toml`
- [ ] Change `VerifierDid` from a unit struct to a struct with `resolver_url`, `client`, `key_cache`, `cache_ttl`
- [ ] Add `VerifierDid::new()` constructor (reads `FLINT_DID_RESOLVER_URL` env; default no-op URL when unset)
- [ ] Add `VerifierDid::with_resolver(url)` constructor for tests
- [ ] Update `parse_did()` / refactor into `resolve_key(did) -> Result<VerifyingKey, SignError>`:
  - Fast path: base64url decode suffix → valid 32-byte key → return immediately
  - Slow path: cache miss → `GET {resolver_url}/v1/did/{did}` → parse `verificationMethod[0].publicKeyBase64Url` → store in cache → return
  - Cache hit: within TTL → return cached key
- [ ] Add `wiremock` to `[dev-dependencies]` in `fke-sign-did/Cargo.toml`
- [ ] Unit test: inline key still works without network (fast path)
- [ ] Unit test: mock resolver returns key → verifier resolves and caches it
- [ ] Unit test: mock resolver returns 404 → `SignError::Invalid`
- [ ] Unit test: cached key is returned without hitting network on second call
- [ ] Unit test: expired TTL triggers re-fetch
- [ ] Existing 8 unit tests still pass (inline key path unchanged)
- [ ] `cargo clippy -p fke-sign-did -- -D warnings` clean
