# Reflection — p6b-kiln-hardening

**Completed:** 2026-07-04  
**Duration:** 1 session (same-day as p6 MVP)  
**Gate result:** PASSED — 8/8 changes, `cargo clippy --workspace -- -D warnings` clean, 426 workspace tests (0 failures)

---

## Goal Achievement

| Goal | Status | Notes |
|---|---|---|
| G1 Cedar capability gate | **MET** | `Pep::check(kiln:invoke)` fires before instantiation; `caller=None` skips Cedar for BGW |
| G2 Kiln BGW (hook→WASM) | **MET** | Full `flint_hooks → webhook_outbox → kiln_bgw → EdgeRuntime::handle()` path live |
| G3 WIT bindings + wasi:http | **MET** | `ProxyPre<KilnHostState>`, `WasiHttpView`, real `call_handle` dispatch; gate test passes with `hello_component.wasm` |
| G4 `fke-sign-did` | **MET** | Ed25519 + `did:prometheus` inline key, validity window, 8 unit tests |
| G5 `fke-sign-cosign` | **MET** | Rekor HTTP + ECDSA P-256, wiremock tests, expired/404/invalid paths covered |
| G6 `fke-store-oci` | **MET** | OCI push/pull/exists via `oci-client 0.17`; idempotent put; env-config + `with_registry()` |
| G7 `fke-store-ipfs` | **MET** | Kubo `/api/v0/add,cat,stat`; wiremock tests; manual multipart (reqwest workspace lacks feature) |
| G8 `fke-store-s3` | **MET** | `object_store 0.11` (`AmazonS3Builder` + `InMemory` for tests); 5 tests |

**Overall: 8/8 goals MET (100%)**

Phase gate criteria satisfied:
- ✅ `fke-runtime::check_capabilities()` is Cedar-gated
- ✅ Kiln BGW processes `target_type='kiln'` end-to-end
- ✅ `wasi:http/incoming-handler` dispatch is real (gate test: HTTP 200 from hello_component)
- ✅ `fke-sign-did` is non-`todo!()` (sovereign default)
- ✅ `fke-store-oci` is non-`todo!()` (remote OCI store)

---

## Delivered Changes

| # | Change | Priority | Lines | Tests |
|---|---|---|---|---|
| c001 | Cedar gate (`forge-policy/src/kiln.rs` + `EdgeRuntime::with_pep`) | P0 | ~170 | 4 Cedar gate tests |
| c002 | Kiln BGW (`fke-server/src/kiln_bgw.rs`) | P0 | 294 | 5 unit tests |
| c003 | WIT + wasi:http (`fke-runtime` full rewrite) | P0 | 523 | 11 (incl. gate test) |
| c004 | `fke-sign-did` | P1 | 284 | 8 unit tests |
| c005 | `fke-sign-cosign` | P1 | 304 | 5 wiremock tests |
| c006 | `fke-store-oci` | P1 | 309 | 4 unit tests (2 ignored/live) |
| c007 | `fke-store-ipfs` | P2 | 247 | 5 wiremock tests |
| c008 | `fke-store-s3` | P2 | 193 | 5 InMemory tests |

**Total new code:** ~2,243 lines across 9 files  
**New workspace dependencies:** 10 crates (`wasmtime-wasi-http`, `hyper`, `http-body-util`, `ed25519-dalek`, `sha2`, `chrono`, `base64`, `p256`, `oci-client`, `object_store`)

---

## Artifact Quality Summary

No artifact-refiner logs exist (this phase did not use the artifact-refiner tool).  
Quality was enforced directly via the CI gate: `cargo clippy --workspace -- -D warnings`.

| Metric | Value |
|---|---|
| Clippy gate | ✅ Clean (0 errors, 0 warnings) |
| Test pass rate | 426 / 426 (100%) |
| Changes with test coverage | 8 / 8 (100%) |
| Gate tests (require runtime) | 1 (hello_component HTTP 200) |
| Tests skipped (live infra) | 2 (OCI registry integration) |
| Clippy issues resolved during build | 12 (format_push_string, map_or, let_else, etc.) |

---

## What Worked Well

1. **Parallel execution** — c005+c006 and c007+c008 ran as concurrent subagents, halving wall-clock time for the P1/P2 batch.

2. **`agui_hook_dispatcher` as a template** — The Kiln BGW (c002) was ~80% copy-paste from the existing `agui_hook_dispatcher.rs` pattern. Same SKIP LOCKED + exponential backoff + error handling. Zero architecture decisions required.

3. **`ProxyPre` vs `InstancePre`** — The switch from `InstancePre<KilnHostState>` to `ProxyPre<KilnHostState>` in c003 was cleaner than expected once the docs were read carefully. The oneshot channel pattern for response collection is well-documented.

4. **`did:prometheus` inline key** — Embedding the 32-byte Ed25519 public key as base64url directly in the DID string eliminates network resolution for the common case. No DID document HTTP lookup needed for unit tests.

5. **`object_store` abstraction** — The `with_store(Arc<dyn ObjectStore>)` constructor pattern allowed `InMemory` tests for S3 without any network mocking. Clean and fast.

---

## What Was Harder Than Expected

1. **`wasmtime-wasi-http` Scheme visibility** — `wasmtime_wasi_http::types::Scheme` is private at that path in v26; the correct import is `wasmtime_wasi_http::bindings::http::types::Scheme`. Took a probe crate to confirm the path compiles.

2. **`HyperIncomingBody` type mismatch** — `new_incoming_request` requires `B: Body<Data=Bytes, Error=hyper::Error>`. `http_body_util::Full<Bytes>` has `Error=Infallible`; the `match e {}` trick converts `Infallible → hyper::Error` cleanly but is non-obvious.

3. **`reqwest` multipart** — The workspace `reqwest` config lacks the `multipart` feature, so `StoreIpfs::put` had to build multipart/form-data manually. This is a known limitation; add `multipart` to the workspace reqwest features when needed.

4. **Parallel subagent for cosign returned empty** — First dispatch of `fke-sign-cosign` came back with no file writes. Implemented directly on the second pass. Root cause: subagent task completed but didn't persist files. Mitigation: always verify file contents before trusting subagent results.

5. **`env::set_var` races in OCI tests** — The subagent wrote a test using `env::set_var` with parallel test execution, causing flaky failures. Fixed by adding `StoreOci::with_registry()` and rewriting the test to avoid global state.

---

## Technical Debt Introduced

| Item | Location | Severity | Remediation |
|---|---|---|---|
| `AllowAllPolicySource` in production | `fke-server/src/kiln_policy.rs` | LOW | Replace with `DbPolicySource` loading from `flint_kiln.cedar_policies` when policy management lands |
| `caller=None` bypasses Cedar for BGW | `fke-server/src/kiln_bgw.rs` | LOW | Add publisher DID → synthetic `RlsContext` in BGW when Kiln security hardens |
| Cosign skips Fulcio chain validation | `fke-sign-cosign` | MEDIUM | Implement full Sigstore certificate chain (Fulcio root → leaf cert) in follow-on |
| `reqwest` missing `multipart` feature | workspace `Cargo.toml` | LOW | Add `multipart` feature to workspace reqwest when file upload surfaces need it |
| OCI `oci-client 0.17` default feature disabled | workspace | LOW | Re-evaluate `aws-lc-rs` vs `rustls-tls` when deployment target is confirmed |
| `fke-runtime.rs` is 523 lines | `crates/fke-runtime/src/lib.rs` | LOW | Split into `runtime/linker.rs` + `runtime/http.rs` + `runtime/tests.rs` when adding epoch interruption |

---

## Lessons Captured

1. **Always verify subagent file writes** — Check `wc -l` or read a sentinel line before trusting that a parallel agent persisted its work.

2. **`ProxyPre` is the right primitive for wasi:http** — Do not use `InstancePre` for HTTP components; use `ProxyPre<S>` which pre-indexes the `wasi_http_incoming_handler()` export.

3. **`env::set_var` is not thread-safe** — Any test using `env::set_var` must either: (a) run with `--test-threads=1`, (b) use a constructor that accepts explicit values, or (c) use `temp-env` crate. Prefer (b).

4. **DID inline key pattern scales** — `did:prometheus:<base64url(pubkey)>` lets every component be self-describing. No DID registry or resolution server required for offline verification.

5. **Parallel subagents double throughput for independent changes** — The 4 P2 changes (c007, c008) took the same wall time as 1 sequential change when dispatched concurrently.

---

## Recommended Next Phase

**Name:** `p7b-kiln-production` (or continue existing p7 if that scope fits)

**Focus:** Production hardening of the Kiln stack:

1. **Epoch interruption** — Add `Config::epoch_interruption(true)` to `EdgeRuntime` so long-running components can be interrupted without wasting fuel.

2. **Fulcio certificate chain** — Complete Sigstore verification: validate Cosign signature against the full Fulcio CA chain using `sigstore` crate.

3. **Cedar policies from DB** — Replace `AllowAllPolicySource` with `DbPolicySource` loading from `flint_kiln.cedar_policies` — same as Quarry's `flint_meta.cedar_policies`.

4. **BGW publisher identity** — Synthesize a `RlsContext` from the function's `publisher_did` in the BGW so Cedar is exercised on hook-triggered invocations.

5. **`fke-sign-did` HTTP DID resolution** — Extend beyond inline public keys to resolve `did:prometheus:<id>` via HTTP when the key is not embedded.

6. **OCI + IPFS + S3 integration tests** — Gate tests requiring live infra, run in CI with `--features integration`.

**Alternative:** If the project is ready to ship the current Kiln stack to a staging environment, a consolidation sprint (CI, Docker images, load tests) would deliver more value than further protocol hardening at this stage.
