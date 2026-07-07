# p6b-c004 Tasks — Ed25519 Signature Verifier

## Tasks

- [ ] Add `ed25519-dalek = "2"`, `sha2 = "0.10"`, `chrono = "0.4"`, `base64 = "0.22"` to `[workspace.dependencies]` (check existing first per constraints)
- [ ] Add those deps to `fke-sign-did/Cargo.toml`
- [ ] Replace `VerifierDid::verify()` `todo!()` with:
  - Parse `manifest.publisher_did` → extract Ed25519 public key (inline `did:prometheus:<base64url>` format)
  - Check `not_before` / `not_after` against current UTC time
  - Compute `sha256(artifact_bytes)` with `sha2::Sha256`
  - `ed25519_dalek::VerifyingKey::verify_strict(message, &signature)`
  - Map errors to `SignError` variants
- [ ] Unit test: valid signature verifies successfully
- [ ] Unit test: expired `not_after` → `SignError::Expired`
- [ ] Unit test: wrong signature → `SignError::Invalid`
- [ ] Unit test: malformed DID → `SignError::Invalid`
- [ ] `cargo clippy -p fke-sign-did -- -D warnings` clean
- [ ] `cargo test -p fke-sign-did` passes
