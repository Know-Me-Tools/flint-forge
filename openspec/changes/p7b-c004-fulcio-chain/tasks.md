# p7b-c004 Tasks — Fulcio Certificate Chain

## Tasks

- [ ] **Audit first:** `cargo search sigstore` — confirm `0.14` API; check `sigstore-verify = "0.10"` as lighter alternative
- [ ] Add chosen crate to `[workspace.dependencies]` (pin to exact minor version)
- [ ] Add dep to `fke-sign-cosign/Cargo.toml`
- [ ] Implement `VerifierCosignFull` using sigstore crate full chain verification
- [ ] Rename existing `VerifierCosign` → `VerifierCosignLegacy`; make `VerifierCosign` a mode-switching wrapper
- [ ] Read `FLINT_COSIGN_MODE` env var: `full` (default) uses Fulcio chain; `legacy` uses existing ECDSA-only path
- [ ] Unit test: `FLINT_COSIGN_MODE=legacy` still passes existing 5 wiremock tests
- [ ] Unit test: `FLINT_COSIGN_MODE=full` rejects a signature with no Fulcio chain → `SignError::Invalid`
- [ ] `cargo clippy -p fke-sign-cosign -- -D warnings` clean
- [ ] `cargo test -p fke-sign-cosign` passes (all existing tests + new Fulcio tests)
