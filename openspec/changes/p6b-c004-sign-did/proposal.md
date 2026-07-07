# p6b-c004 — Ed25519 / did:prometheus Signature Verifier

**Phase:** 6b — Kiln Hardening
**Priority:** P1
**Depends on:** none

## What this change delivers

Replaces the `todo!()` body of `VerifierDid::verify()` with a real Ed25519
signature verification against a `did:prometheus` DID document. This is the
default sovereign verifier — no external PKI required.

## Design

### Verification flow

1. Parse `manifest.publisher_did` → extract Ed25519 public key bytes.
   - Support: `did:prometheus:<base64url-pubkey>` inline format (no resolution needed for tests)
   - Extend later: HTTP resolution for did:prometheus DID documents
2. Check `manifest.not_before` / `manifest.not_after` validity window against `chrono::Utc::now()`
3. Compute `message = sha256(artifact) || manifest.content_digest.as_bytes()`
4. Verify `ed25519_dalek::VerifyingKey::verify_strict(message, signature)`

### New deps

```toml
# fke-sign-did/Cargo.toml
ed25519-dalek = { version = "2", features = ["rand_core"] }
sha2 = "0.10"
chrono = { version = "0.4", features = ["serde"] }
base64 = "0.22"
anyhow = { workspace = true }
```

These must be added to `[workspace.dependencies]` where absent.

### Error mapping

| Condition | `SignError` variant |
|---|---|
| DID parse failure | `SignError::Invalid` |
| Validity window expired | `SignError::Expired` |
| Signature mismatch | `SignError::Invalid` |
| Missing/unsigned | `SignError::Unsigned` |
