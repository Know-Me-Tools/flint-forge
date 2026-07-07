# p6b-c005 Tasks — Sigstore/Cosign Verifier

## Tasks

- [ ] Add `p256 = "0.13"` to `[workspace.dependencies]`; add `p256`, `reqwest`, `sha2`, `base64`, `chrono`, `anyhow` to `fke-sign-cosign/Cargo.toml`
- [ ] Add `wiremock` + `tokio` as dev-deps in `fke-sign-cosign/Cargo.toml`
- [ ] Read `FLINT_REKOR_URL` env var; default to `https://rekor.sigstore.dev`
- [ ] Implement `VerifierCosign::verify()`:
  - Compute `sha256(artifact)` hex digest
  - POST to Rekor `/api/v1/log/entries/retrieve` with digest
  - Parse log entry → extract `sig` (base64) + `publicKey` (base64 PEM)
  - Verify ECDSA P-256 signature via `p256::ecdsa::VerifyingKey::verify()`
  - Check `manifest.not_before`/`not_after` validity window
  - Map errors to `SignError` variants
- [ ] Unit test: mock Rekor server via `wiremock` → valid signature verifies
- [ ] Unit test: mock returns 404 → `SignError::Unsigned`
- [ ] Unit test: mock returns invalid signature bytes → `SignError::Invalid`
- [ ] Unit test: expired validity window → `SignError::Expired`
- [ ] `cargo clippy -p fke-sign-cosign -- -D warnings` clean
