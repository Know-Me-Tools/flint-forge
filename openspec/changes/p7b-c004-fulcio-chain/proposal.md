# p7b-c004 — Fulcio Certificate Chain (Sigstore end-to-end verification)

**Phase:** 7b — Kiln Production Hardening
**Priority:** P1
**Depends on:** none (additive to existing `fke-sign-cosign`)
**Blocks:** nothing

## What this change delivers

Upgrades `fke-sign-cosign::VerifierCosign` from "fetch Rekor + verify raw
ECDSA P-256" to full Sigstore verification including Fulcio certificate chain
validation. After this change, a Cosign signature is only accepted if the
signing certificate chains to the Fulcio CA trusted root.

## Design

### Dependency

```toml
# Workspace
sigstore = "0.14"
```

Audit the `0.14` API before coding. If `sigstore::cosign::CosignCapabilities`
is not stable in `0.14`, use `sigstore-verify = "0.10"` instead — it is a
smaller, more stable extraction.

### Verification flow (high level)

```rust
use sigstore::cosign::{ClientBuilder, SignatureLayer};

let client = ClientBuilder::default()
    .with_rekor_pub_key(...)   // or fetch lazily
    .with_fulcio_certs(...)    // from Sigstore TUF trust root
    .build()?;

let layers: Vec<SignatureLayer> = client
    .trusted_signature_layers(auth, source, image)
    .await?;

// At least one layer must satisfy the predicate
for layer in layers {
    if layer.simple_signing.optional.get("subject") == Some(publisher_did) {
        return Ok(());
    }
}
Err(SignError::Invalid)
```

### Backward compatibility

The existing `VerifierCosign` implementation (Rekor + raw ECDSA) stays as
`VerifierCosignLegacy` for environments without Fulcio CA access. The new
Fulcio-gated verifier becomes the default `VerifierCosign`.

A `FLINT_COSIGN_MODE=legacy|full` env var (default `full`) lets operators
opt back to the legacy path.

## Risk

`sigstore 0.14` API may break between minor versions. Pin to `=0.14.x` exact
version in workspace once the implementation is confirmed working.
