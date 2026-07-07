# Goals — p7b-kiln-production

## Phase Summary

Production-harden the Flint Kiln WASM edge runtime. The p6b phase delivered
all core infrastructure (Cedar gate, BGW, wasi:http dispatch, signers, stores).
p7b closes the remaining correctness, security, and operability gaps before
Kiln can be recommended for production deployments.

Seeded from: `p6b-kiln-hardening/reflection.md` → "Recommended Next Phase"

---

## Changes (6 planned)

### P0 — Must ship

- **G1 — p7b-c001-epoch-interruption:**
  Add `Config::epoch_interruption(true)` to `EdgeRuntime`. Spawn a background
  tokio task that ticks the wasmtime epoch on a configurable interval (default
  10 ms). Set `Store::set_epoch_deadline(1)` per invocation so components that
  run past the deadline are interrupted cleanly — no polling fuel waste, no
  runaway components. Pairs with the existing fuel limit as a defence-in-depth
  timeout.

- **G2 — p7b-c002-cedar-db-policies:**
  Replace `AllowAllPolicySource` in `fke-server/src/kiln_policy.rs` with a
  real `DbPolicySource` that loads Cedar policies from a new
  `flint_kiln.cedar_policies` Postgres table (same schema as
  `flint_meta.cedar_policies` used by the Quarry). Wire
  `CedarPolicyEngine::new(DbPolicySource::new(pool))` in `fke-server/src/main.rs`.
  Requires migration `0009_flint_kiln_cedar_policies.sql`.

- **G3 — p7b-c003-bgw-publisher-identity:**
  Synthesize an `RlsContext` from the function's `publisher_did` in
  `kiln_bgw::invoke_function()` so the Cedar gate is exercised on
  hook-triggered invocations (currently `caller=None` bypasses Cedar for BGW
  calls). The `keto_subject` field is set to the `publisher_did`; `role` is
  `"kiln_publisher"`; `claims_json` contains `{"sub": "<publisher_did>"}`.

### P1 — Should ship

- **G4 — p7b-c004-fulcio-chain:**
  Complete Sigstore/Cosign verification in `fke-sign-cosign`: validate the
  Cosign signature against the full Fulcio certificate chain. Add
  `sigstore = "0.9"` (or latest) as a workspace dep. Use
  `sigstore::cosign::CosignCapabilities` to perform end-to-end verification
  including transparency log consistency proof.

- **G5 — p7b-c005-did-http-resolution:**
  Extend `fke-sign-did::VerifierDid` to support HTTP DID document resolution
  when the DID string does not embed a key inline. For DIDs of the form
  `did:prometheus:<id>` (no key bytes), resolve via
  `GET {FLINT_DID_RESOLVER_URL}/v1/did/{did}` and extract the Ed25519
  `verificationMethod[0].publicKeyBase64Url`. Cache resolved keys with a TTL
  (default 5 min via `moka` or `std::sync::Mutex<HashMap<String, (VerifyingKey, Instant)>>`).

### P2 — Ship if capacity allows

- **G6 — p7b-c006-integration-tests:**
  Integration test harness for OCI, IPFS, and S3 stores gated behind
  `--features integration`. Tests use `testcontainers` (or env-based skip) to
  spin up a local registry/daemon. Gate: `cargo test -p fke-store-oci
  --features integration` passes against a local `registry:2` container.

---

## Phase Complete When (MVP gate)

- [ ] Epoch interruption wired; a component that loops forever is interrupted within 100 ms
- [ ] Cedar policies loaded from `flint_kiln.cedar_policies` (real DB, not allow-all stub)
- [ ] BGW passes `caller = Some(publisher_rls)` to `EdgeRuntime::handle()` so Cedar fires
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean

---

## Dependencies

### All resolved (from p6b)
- `EdgeRuntime` — ✅ p6-c001
- `KilnHostState` + wasi:http — ✅ p6b-c003
- `CedarPolicyEngine` + `forge-policy` Pep — ✅ p6b-c001
- `kiln_bgw` — ✅ p6b-c002
- `flint_kiln` schema — ✅ p6-c002 (migration 0008)
- `fke-sign-cosign` base — ✅ p6b-c005
- `fke-sign-did` base — ✅ p6b-c004

### New dependency: `sigstore` crate (G4)
OQ-P7B-1: Confirm `sigstore = "0.9"` API stability before adding as workspace dep.
The crate is maintained by the Sigstore project (Rust SIG) but has active churn.
Pin to a specific minor version; budget for API fixes during p7b execution.

### New dependency: `moka` or `std` TTL cache (G5)
OQ-P7B-2: `moka` is the canonical async cache; `std` + `Instant` is fine for
a simple two-field TTL. Prefer `moka` if it's already a transitive dep, else use std.

### New dependency: `testcontainers` (G6)
OQ-P7B-3: `testcontainers = "0.23"` + `testcontainers-modules` for
`registry`, `minio`, and kubo images.
