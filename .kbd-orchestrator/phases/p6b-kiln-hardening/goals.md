# Goals — p6b-kiln-hardening

## Phase Summary

Complete the Flint Kiln WASM edge runtime. The p6 MVP landed the execution engine, registry, and local store. p6b closes the remaining gaps: security gates, remote artifact stores, WIT interface contract, and the hook-to-Kiln BGW that makes the full `flint_hooks → Kiln → WASM function` path live end-to-end.

## Changes (8 total)

### P0 — Must ship (system is incomplete without these)

- **G1 — p6b-c001 Cedar capability gate:** Enforce `Capability` intersection against Cedar policy before `fke-runtime::EdgeRuntime::handle()` instantiates a component. A function declared with `[Db, Llm]` that the Cedar policy only grants `[Db]` must be rejected at the gate, not inside the component. Lives in `fke-runtime` using the `forge-policy` `Pep` trait already in the codebase.

- **G2 — p6b-c002 Kiln BGW:** Background worker that polls `flint.webhook_outbox WHERE target_type = 'kiln'` (seeded in p7-c001), resolves the function via `fke-registry`, loads WASM from store, invokes `fke-runtime`, and marks the outbox entry `delivered` or `failed`. Lives in `fke-server` as a `tokio::spawn` loop, same pattern as `agui_hook_dispatcher` in `fdb-gateway`.

- **G3 — p6b-c003 WIT contract freeze:** Define `flint:host@0.1.0` WIT interface in `crates/fke-domain/wit/`. Generate Rust host bindings with `wit-bindgen`. Wire `wasi:http/incoming-handler@0.2` into `fke-runtime` so the HTTP dispatch stub becomes a real function call. Freeze the interface; breaking changes require a version bump.

### P1 — Should ship

- **G4 — p6b-c004 `fke-sign-did`:** Replace `todo!()` in `crates/fke-sign-did/src/lib.rs` with Ed25519 signature verification against a `did:prometheus` DID document. This is the default sovereign verifier — no external registry required.

- **G5 — p6b-c005 `fke-sign-cosign`:** Replace `todo!()` in `crates/fke-sign-cosign/src/lib.rs` with Sigstore Cosign verification. Fetches the transparency log entry and checks the ECDSA signature against the Rekor bundle.

- **G6 — p6b-c006 `fke-store-oci`:** Replace `todo!()` in `crates/fke-store-oci/src/lib.rs` with an OCI registry adapter. Pulls WASM artifacts stored as OCI layers (content-addressed by digest). Uses the `oci-client` or `oci-distribution` crate.

### P2 — Ship if capacity allows

- **G7 — p6b-c007 `fke-store-ipfs`:** Replace `todo!()` in `crates/fke-store-ipfs/src/lib.rs` with a Kubo HTTP API adapter (`/api/v0/cat`, `/api/v0/add`). CID is the `ContentId`.

- **G8 — p6b-c008 `fke-store-s3`:** Replace `todo!()` in `crates/fke-store-s3/src/lib.rs` with an S3/R2 adapter using `aws-sdk-s3` or `object_store`.

## Phase Complete When (MVP gate)

- [ ] `fke-runtime::check_capabilities()` calls into Cedar policy, not just list comparison
- [ ] Kiln BGW processes a `target_type='kiln'` outbox entry end-to-end without panicking
- [ ] `wasi:http/incoming-handler` dispatch is real (not stubbed)
- [ ] At least one signature verifier is non-`todo!()` (`fke-sign-did` preferred)
- [ ] At least one remote store is non-`todo!()` (`fke-store-oci` preferred)
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean

## Dependencies

### All resolved (gates cleared from p6 MVP)
- `flint_kiln` Postgres schema → migration 0008 delivered ✅
- `fke-runtime` `EdgeRuntime` → p6-c001 delivered ✅
- `fke-registry` `PgRegistry` → p6-c002 delivered ✅
- `fke-server` compose root → p6-c003 delivered ✅
- `webhook_outbox target_type='kiln'` routing stub → p7-c001 delivered ✅

### Open questions
- OQ-P6B-1: `wit-bindgen` CLI version to pin — use latest stable (0.36+)
- OQ-P6B-2: Sigstore Rekor transparency log endpoint — default `https://rekor.sigstore.dev`; configurable via env `FLINT_REKOR_URL`
- OQ-P6B-3: IPFS Kubo endpoint — default `http://localhost:5001`; configurable via env `FLINT_IPFS_URL`
