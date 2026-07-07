# p7b-c005 — DID HTTP Resolution with TTL Cache

**Phase:** 7b — Kiln Production Hardening
**Priority:** P1
**Depends on:** none (additive to existing `fke-sign-did`)
**Blocks:** nothing

## What this change delivers

Extends `fke-sign-did::VerifierDid` to support `did:prometheus` DIDs that
don't embed the key inline. When the DID suffix is not a valid 32-byte
base64url public key, the verifier falls back to HTTP resolution against a
configurable DID resolver, caching results with a 5-minute TTL.

## Design

### DID resolution endpoint

```
GET {FLINT_DID_RESOLVER_URL}/v1/did/{did}
```

Response shape:
```json
{
  "verificationMethod": [
    {
      "type": "Ed25519VerificationKey2020",
      "publicKeyBase64Url": "<base64url-encoded 32-byte key>"
    }
  ]
}
```

Default resolver URL: `https://did.flint.example.com` (set via
`FLINT_DID_RESOLVER_URL`; tests inject a mock URL).

### Updated `VerifierDid` struct

```rust
pub struct VerifierDid {
    resolver_url: String,
    client: reqwest::Client,
    key_cache: Mutex<HashMap<String, (VerifyingKey, Instant)>>,
    cache_ttl: Duration,  // default 5 min
}
```

### Updated `parse_did` logic

```
1. Try to base64url-decode the DID suffix → 32 bytes → Ed25519 key  (fast path, no network)
2. If that fails → call resolver HTTP endpoint → parse key → cache result
3. On cache hit (within TTL) → return cached key directly
```

### New `reqwest` dep in `fke-sign-did/Cargo.toml`

`reqwest` is already a workspace dep.

### TTL cache

`std::sync::Mutex<HashMap<String, (VerifyingKey, Instant)>>` with `Instant`
comparison. Avoids the `moka` dep for now. Eviction on read (lazy): if
`now() - cached_at > ttl`, refetch.
