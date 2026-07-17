# p16-c002 Tasks — Kiln Supply-Chain Trust

## Tasks

- [x] Add `signature: ComponentSignature` field to `FunctionManifest` (`crates/fke-domain/src/lib.rs:39`)
- [x] Add `sha2` to `[workspace.dependencies]`
- [x] Add `fke-sign-did` and `fke-sign-cosign` as dependencies of `fke-server`
- [x] Wire signature verification into `register_function` (reject unsigned/invalid at register)
- [x] Wire signature verification into `invoke_impl` (reject at invoke; decide verify-every-call vs. verified-at-register + periodic re-check)
- [x] Replace `sha256_hex` in `crates/fke-registry/src/lib.rs:108` with `sha2::Sha256`
- [x] Replace `content_id_for` in `crates/fke-store-fs/src/lib.rs:37` with `sha2::Sha256`
- [x] Replace the issuer-substring Fulcio check in `crates/fke-sign-cosign/src/lib.rs:168` with real chain + SCT + OIDC-identity verification
- [x] Update existing `sha256_hex_stable_for_same_input` test to assert against real SHA-256 vectors
- [x] Integration test: unsigned component rejected at register and invoke
- [x] Integration test: tampered-bytes component rejected (hash/signature mismatch)
- [x] Integration test: validly signed component registers and invokes successfully end-to-end
- [x] Unit test: cosign verification rejects issuer-substring-match-but-invalid-chain cert
- [x] Document the manifest signature field as a Kiln ABI change (post-v1.0.0-freeze review)
- [x] `cargo clippy --workspace -- -D warnings` clean
- [x] `cargo test --workspace` passes
