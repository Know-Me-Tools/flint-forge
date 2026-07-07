# p6b-c005 — Sigstore/Cosign OCI Signature Verifier

**Phase:** 6b — Kiln Hardening
**Priority:** P1
**Depends on:** none

## What this change delivers

Replaces `VerifierCosign::verify()` `todo!()` with Sigstore Cosign verification.
Fetches the Rekor transparency log entry and verifies the ECDSA P-256 signature
over the artifact digest.

## Design (MVP scope — verify without full Fulcio chain)

### Verification flow

1. Compute `sha256(artifact)` → hex digest
2. POST to Rekor search endpoint: `POST /api/v1/log/entries/retrieve` with `{ "hash": "sha256:<digest>" }`
3. Parse the transparency log entry, extract the `sig` (base64 ECDSA P-256 signature) and `publicKey` (base64 PEM)
4. Verify ECDSA P-256 signature over `sha256(artifact_bytes)` using `p256::ecdsa::VerifyingKey`
5. Check `manifest.not_before` / `manifest.not_after` validity window

### Configuration

`FLINT_REKOR_URL` env var (default: `https://rekor.sigstore.dev`). Unit tests use a
`wiremock` mock server.

### New deps

```toml
# fke-sign-cosign/Cargo.toml
reqwest = { workspace = true }
sha2 = "0.10"
p256 = { version = "0.13", features = ["ecdsa"] }
base64 = "0.22"
chrono = "0.4"
wiremock = { workspace = true }  # dev-dep
tokio = { workspace = true }     # async tests
anyhow = { workspace = true }
```
