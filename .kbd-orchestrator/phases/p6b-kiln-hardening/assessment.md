# Assessment — p6b-kiln-hardening

**Phase:** p6b-kiln-hardening  
**Assessed:** 2026-07-04  
**Assessor:** opencode  
**Previous phase:** p6-kiln-runtime (4/4 MVP done; clippy clean; 387 tests passing)

---

## Codebase Inventory

### What was built in p6 (foundation for p6b)

| Crate | State | Key API |
|---|---|---|
| `fke-runtime` | **WIRED** | `EdgeRuntime::new()`, `load_wasm()`, `handle()`, `check_capabilities()` |
| `fke-registry` | **WIRED** | `PgRegistry::resolve()`, `PgComponentStore::put/get/exists()` |
| `fke-server` | **WIRED** | `/functions/v1/{name}` invoke, `/admin/functions` register/list |
| `fke-store-fs` | **WIRED** | Content-addressed local store, 5 passing tests |
| `flint_kiln` schema | **WIRED** | `functions`, `artifacts`, `invocations` tables (migration 0008) |
| `webhook_outbox target_type='kiln'` | **WIRED** | Queued in outbox (p7-c001 stub), BGW not yet present |

### What remains as `todo!()` stubs

| Crate | File | Gap |
|---|---|---|
| `fke-sign-did` | `src/lib.rs:19` | `VerifierDid::verify()` — entire body is `todo!()` |
| `fke-sign-cosign` | `src/lib.rs:19` | `VerifierCosign::verify()` — entire body is `todo!()` |
| `fke-store-oci` | `src/lib.rs` | All 3 trait methods `todo!()` |
| `fke-store-ipfs` | `src/lib.rs` | All 3 trait methods `todo!()` |
| `fke-store-s3` | `src/lib.rs` | All 3 trait methods `todo!()` |
| `fke-runtime` | `src/lib.rs:143` | HTTP dispatch after `instantiate_async` — comment says "stub response" |

---

## Gap Analysis by Goal

### G1 — Cedar Capability Gate (P0)

**Current state:** `check_capabilities()` in `fke-runtime` is a pure Rust list comparison (`granted.contains(cap)`). It does NOT call the Cedar policy engine.

**Gap:**
- `forge-policy` `Pep::check()` exists and is used in `fdb-gateway`. The `CedarPolicyEngine` is live.
- `fke-runtime`'s `handle()` calls `check_capabilities()` but does not inject a `Pep`. No `Pep` in `EdgeRuntime` state.
- Cedar policy resource namespace for Kiln functions (`kiln:invoke`) does not exist in `forge-policy` yet.

**Work required:**
- Add a `kiln.rs` to `forge-policy` defining `KILN_RESOURCE`, `KILN_INVOKE` constants (mirrors `a2ui.rs`)
- Add `pep: Option<Arc<dyn Pep>>` to `EdgeRuntime`
- Replace `check_capabilities()` list comparison with `pep.check(who, kiln_request)` + capability presence check
- Update `fke-server` to inject a `CedarPolicyEngine` into `EdgeRuntime::new()`

**Effort:** Medium — 3 files changed, pattern identical to `fdb-gateway`'s Cedar wiring.

---

### G2 — Kiln BGW (P0)

**Current state:** `fke-server/src/main.rs` has no background worker. The `flint.webhook_outbox` rows with `target_type='kiln'` accumulate and are never drained.

**Gap:** Missing a polling loop that:
1. SELECTs from `flint.webhook_outbox WHERE target_type='kiln' AND status IN ('pending','retrying')` with SKIP LOCKED
2. Resolves the function from `fke-registry`
3. Calls `fke-runtime::EdgeRuntime::handle()`
4. Marks entry `delivered` or applies exponential backoff on failure

**Pattern available:** `fdb-gateway/src/agui_hook_dispatcher.rs` (built in p7-c002) is the exact same BGW pattern — poll + SKIP LOCKED + exponential backoff.

**Work required:**
- Create `crates/fke-server/src/kiln_bgw.rs` modelled on `agui_hook_dispatcher.rs`
- Add `hook_payload` column reading from `webhook_outbox` to extract function name
- Wire `spawn(pool, runtime)` in `fke-server/src/main.rs`

**Effort:** Low-medium — direct port of existing BGW pattern.

---

### G3 — WIT Contract & Real HTTP Dispatch (P0)

**Current state:**
- `examples/hello-component/wit/world.wit` shows a working `wasi:http/incoming-handler` example — bindings generated via `wit-bindgen-rt = "0.44.0"`.
- `fke-runtime::handle()` instantiates the component but then stubs the response: `let _ = (instance, request); Ok(KilnResponse { status: 200, body: b"ok".to_vec() })`.
- No `fke-domain/wit/` directory exists — no `flint:host` interface defined.

**Gap:**
- The WIT interface `flint:host@0.1.0` needs to be authored in `crates/fke-domain/wit/`
- Host binding generation: `wit-component` + `wit-bindgen` must be added to `fke-runtime/build.rs`
- `fke-runtime::handle()` must call the `wasi:http/incoming-handler.handle()` export via the generated typed interface instead of the current stub
- `wasmtime-wasi-http` must be added as a dep (was removed from the initial implementation to avoid the missing-crate error — now needs to land)

**Work required:**
- Write `crates/fke-domain/wit/flint-host.wit` — declare `flint:host@0.1.0` world with `import wasi:http/incoming-handler@0.2.0`
- Add `wit-bindgen = "0.44"` and `wasmtime-wasi-http = "26"` to workspace deps
- Add `build.rs` to `fke-runtime` that calls `wit-bindgen`
- Replace the stub response in `handle()` with real `incoming_handler::call_handle()`

**Effort:** High — WIT authoring + build.rs + wasmtime HTTP integration.

---

### G4 — `fke-sign-did` Ed25519 Verifier (P1)

**Current state:** Entire `verify()` body is `todo!()`. No deps in Cargo.toml beyond `fke-domain`, `fke-ports`, `async-trait`.

**Gap:** Need Ed25519 signature verification. The `FunctionManifest` carries `publisher_did`, `content_digest`, and `not_before`/`not_after` validity window.

**Work required:**
- Add `ed25519-dalek = "2"` (workspace dep) + `serde_json` to `fke-sign-did/Cargo.toml`
- Implement `VerifierDid::verify()`:
  1. Parse `publisher_did` to extract the Ed25519 public key (DID document inline or resolved)
  2. Verify signature bytes over `sha256(artifact) + manifest.content_digest`
  3. Check `not_before` / `not_after` validity window against `now()`

**Effort:** Medium — standard Ed25519 verify; DID parsing is the tricky part.

---

### G5 — `fke-sign-cosign` Sigstore Verifier (P1)

**Current state:** Entire `verify()` body is `todo!()`. No Sigstore deps.

**Gap:** Sigstore Cosign verification requires HTTP calls to the Rekor transparency log.

**Work required:**
- Add `reqwest` (already workspace dep) + `serde_json` to `fke-sign-cosign/Cargo.toml`
- Implement `VerifierCosign::verify()`:
  1. Fetch Rekor log entry for the artifact digest
  2. Verify the ECDSA P-256 signature from the bundle against the artifact
  3. Confirm the certificate chain roots to Sigstore's Fulcio CA
  4. Check validity window from manifest

**Effort:** High — Rekor REST API + certificate validation. Can scope to "fetch + parse + verify ECDSA" without full Fulcio chain validation for MVP.

---

### G6 — `fke-store-oci` (P1)

**Current state:** All 3 trait methods `todo!()`. No OCI deps.

**Gap:** Need an OCI registry client for content-addressed layer pulls.

**Work required:**
- Add `oci-client = "0.14"` (or `oci-distribution`) to workspace deps + `fke-store-oci/Cargo.toml`
- Implement `put()` as OCI layer push (manifest + config + layer)
- Implement `get()` as OCI layer pull by digest
- Implement `exists()` as HEAD against the registry manifest

**Effort:** Medium — `oci-client` crate has a clear API; auth config via env vars.

---

### G7 — `fke-store-ipfs` (P2)

**Current state:** All 3 trait methods `todo!()`. No IPFS deps.

**Gap:** Kubo HTTP API — add and cat endpoints.

**Work required:**
- Add `reqwest` (already workspace dep) to `fke-store-ipfs/Cargo.toml`
- `put()` → POST to `/api/v0/add`, parse CID from response
- `get()` → POST to `/api/v0/cat?arg={cid}`
- `exists()` → POST to `/api/v0/stat?arg={cid}`

**Effort:** Low — pure HTTP calls, no complex dependencies.

---

### G8 — `fke-store-s3` (P2)

**Current state:** All 3 trait methods `todo!()`. No S3 deps.

**Gap:** S3/R2 object store.

**Work required:**
- Add `object_store = "0.11"` (preferred — supports AWS S3, R2, GCS) to workspace deps
- Implement put/get/exists via `object_store::ObjectStore` trait

**Effort:** Low-medium — `object_store` crate has idiomatic Rust API.

---

## Risk Register

| Risk | Likelihood | Severity | Mitigation |
|---|---|---|---|
| `wasmtime-wasi-http` API mismatch with v26 | MEDIUM | HIGH | Check the `wasmtime-wasi-http` v26 crate docs before writing `handle()` — may need `WasiHttpView` impl on `KilnHostState` |
| `wit-component` / `wit-bindgen` version conflict with hello-component's `wit-bindgen-rt = "0.44.0"` | LOW | MEDIUM | Pin `wit-bindgen = "0.44"` to match existing usage in `examples/` |
| Sigstore Rekor is rate-limited or unavailable in CI | MEDIUM | LOW | Gate the Cosign test behind `#[cfg(feature = "integration")]` — unit tests mock the HTTP layer |
| `oci-client` crate registry auth config variance | LOW | MEDIUM | Support `KILN_OCI_REGISTRY`, `KILN_OCI_USER`, `KILN_OCI_TOKEN` env vars; anonymous pull for public registries |
| Ed25519 DID document resolution adds network dep | MEDIUM | MEDIUM | Support both: (a) inline public key in DID fragment, (b) HTTP resolution. Default to inline for tests. |

---

## Dependency Map

```
p6b-c001 (Cedar gate)    ─────────────────────── independent
p6b-c002 (Kiln BGW)      ─────────── depends on p6b-c001 (gate must exist to call before invoke)
p6b-c003 (WIT bindings)  ─────────────────────── independent; unblocks real HTTP dispatch
p6b-c004 (sign-did)      ─────────────────────── independent
p6b-c005 (sign-cosign)   ─────────────────────── independent (but needs reqwest)
p6b-c006 (store-oci)     ─────────────────────── independent
p6b-c007 (store-ipfs)    ─────────────────────── independent
p6b-c008 (store-s3)      ─────────────────────── independent
```

All changes except p6b-c002 are independent. **Optimal execution order:** c001 → c002 in sequence; c003–c008 in parallel.

---

## Assessment Summary

| Goal | Gap Size | Effort | Ready to build? |
|---|---|---|---|
| p6b-c001 Cedar gate | Small — pattern exists in fdb-gateway | Medium | ✅ Yes |
| p6b-c002 Kiln BGW | Small — port of agui_hook_dispatcher | Low-Med | ✅ Yes (after c001) |
| p6b-c003 WIT bindings | Large — new WIT file + build.rs + wasmtime-wasi-http | High | ✅ Yes |
| p6b-c004 sign-did | Medium — Ed25519 + DID parsing | Medium | ✅ Yes |
| p6b-c005 sign-cosign | Large — Rekor HTTP + ECDSA | High | ✅ Yes (scope MVP) |
| p6b-c006 store-oci | Medium — oci-client crate | Medium | ✅ Yes |
| p6b-c007 store-ipfs | Small — Kubo HTTP API | Low | ✅ Yes |
| p6b-c008 store-s3 | Small — object_store crate | Low-Med | ✅ Yes |

**No external blockers.** All 8 changes are implementable against the current codebase. Workspace tooling (`cargo`, `wit-bindgen`, `wit-component`) is already in place from `examples/hello-component`.

**Handoff to plan:** Focus first on the P0 triad (c001 Cedar gate → c002 BGW → c003 WIT). The P1 changes (c004–c006) can run in parallel once the gate and BGW are wired.
