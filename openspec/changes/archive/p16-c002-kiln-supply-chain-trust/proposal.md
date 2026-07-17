# p16-c002 — Kiln Supply-Chain Trust (Signatures + Real Hashing)

**Phase:** 16 — Production Remediation
**Priority:** P0 (blocks any production claim)
**Depends on:** none

## What this change delivers

- A signature on `FunctionManifest`, verified at both `POST /admin/functions`
  (register) and `POST /functions/v1/<name>` (invoke) ingress.
- `fke-server` depends on `fke-sign-did` and `fke-sign-cosign` and actually
  calls them — today neither is a dependency.
- Real `sha2::Sha256` content-addressing, replacing the FNV-style pseudo-hash.
- Real cosign Fulcio chain/SCT/OIDC-identity verification, replacing a
  substring match on the issuer DN.

## Problem

`crates/fke-server/src/main.rs::invoke_impl` (`:172-284`) goes
resolve → load → instantiate → invoke with **no signature check anywhere**.
`FunctionManifest` (`crates/fke-domain/src/lib.rs:39-47`) has no signature
field, so there is nothing to verify against even if a check were added.
`fke-sign-did` and `fke-sign-cosign` contain real, sound crypto (Ed25519
`verify_strict`, real Rekor + ECDSA P-256) but are dead code from the server's
perspective — `grep` for dependents returns nothing.

Separately, `fke-registry::sha256_hex` (`:108-122`) is a deterministic
pseudo-hash ("FNV + length") labeled `sha256:` with a `// TODO: replace with
sha2::Sha256` comment already in the code. `fke-store-fs::content_id_for` has
the identical problem. This means the content-addressing that backs
integrity/dedup on the live path is not cryptographic and is trivially
collidable.

`fke-sign-cosign`'s Fulcio check (`:168-175`) is
`issuer.contains("fulcio") || issuer.contains("sigstore")` — a self-signed
certificate whose issuer CN happens to contain that substring passes with **no
chain-to-root, SCT, or OIDC-identity binding**.

## Design

### 1. Manifest signature field — resolved by investigation

`fke_ports::SignatureVerifier::verify(manifest, signature: &[u8], artifact)`
already takes the signature as a **separate** parameter, not embedded in
`FunctionManifest` — so no trait change was needed. Added
`signature_b64: Option<String>` to `FunctionManifest` instead: required and
used for `did:prometheus:`-scheme manifests (`VerifierDid` needs the blob
explicitly); ignored for Cosign-scheme manifests, since `VerifierCosign`
already fetches its signature material from Rekor keyed by `content_digest`
(confirmed by reading its `verify()` — the passed `_sig` parameter is unused).
`forge-cli fn register` gained a `--signing-key <path>` flag (raw 32-byte
Ed25519 seed) so the existing CLI-driven registration flow keeps working once
verification is mandatory — without this, every `forge-cli` registration
would be rejected as unsigned, a regression this change must not introduce.

### 2. Wire verification into the server

- `fke-server/Cargo.toml`: added `fke-sign-did` and `fke-sign-cosign` as
  dependencies; both verifiers constructed once in `KilnState` (not per-call —
  `VerifierDid` holds a TTL key cache that must persist).
- `register_function`: verifies before `store.put`/registry upsert; rejects
  (403) on failure.
- `invoke_impl`: re-verifies on every **cold cache-load** (`!runtime.is_loaded`)
  — the same lifecycle as the WASM-bytes cache itself — rather than on every
  single invoke. This independently closes the "no verification at execution"
  gap without adding a full verification cost (a Rekor HTTP round-trip, for
  Cosign) to every hot-path request.

### 3. Real hashing — done

`sha2` was already a `[workspace.dependencies]` entry (unlike the stale "TODO:
once sha2 is a workspace dep" comment implied); added it to `fke-registry` and
`fke-store-fs`'s own `Cargo.toml` and replaced both `sha256_hex`/
`content_id_for` pseudo-hash implementations with `Sha256::digest`.

### 4. Real cosign chain validation

Implemented real cryptographic chain verification: the Rekor-supplied leaf
certificate's signature is verified against a **pinned Sigstore Fulcio
intermediate CA**, and the intermediate against a **pinned Fulcio root CA** —
both fetched from `sigstore/root-signing` (the authoritative source) and
cross-checked with `openssl verify` before embedding, plus a regression test
(`pinned_intermediate_chains_to_pinned_root`) that re-verifies this at test
time. This replaces the substring match entirely.

**Deliberately not implemented, and out of this change's scope** (a follow-up
task was filed): SCT (Signed Certificate Timestamp) verification, and binding
the leaf's embedded OIDC identity to an operator-configurable allowlist. The
`sigstore` crate family was considered instead of hand-rolling chain
verification, but rejected for this pass: it bundles its own TUF-based trust
root fetching, which is a runtime network-dependency change with operational
implications (air-gapped Kiln deployments) beyond this change's scope to
evaluate and adopt responsibly.

## Verification (gate)

- Integration test: register + invoke an **unsigned** component → rejected at
  both ingress points.
- Integration test: register a component with a **tampered byte** after
  signing → rejected (hash/signature mismatch).
- Integration test: register + invoke a **validly signed** component → runs
  successfully end-to-end (real WASI-HTTP response).
- Unit tests: `sha256_hex`/`content_id_for` match real `sha2::Sha256` reference
  vectors (not just internal stability).
- Unit tests: cosign verification rejects a cert with a matching issuer
  substring but a chain that doesn't cryptographically validate against the
  pinned root; a positive regression test confirms the pinned intermediate
  genuinely chains to the pinned root.
