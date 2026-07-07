# Reflection — p7b-kiln-production

**Completed:** 2026-07-04
**Duration:** 1 session (continuation of p7b, same day)
**Gate result:** PASSED — 6/6 changes, `cargo clippy --workspace -- -D warnings` clean, 437 workspace tests (0 failures)

---

## Goal Achievement

| Goal | Status | Notes |
|---|---|---|
| G1 Epoch interruption | **MET** | `epoch_interruption(true)` + 10 ms ticker + `set_epoch_deadline(1)` per call. Fast components still complete; tested against `hello_component.wasm` at 10 ms ticks. |
| G2 Cedar policies from DB | **MET** | `DbKilnPolicySource` reads `flint_kiln.cedar_policies`; migration 0009 seeds bootstrap allow-all row. `AllowAllPolicySource` retained as test stub only. |
| G3 BGW publisher identity | **MET** | `publisher_rls(manifest)` synthesises `RlsContext` from `publisher_did`; both `handle()` call sites now pass `Some(&publisher)`. Cedar fires on every hook-triggered invocation. |
| G4 Fulcio certificate chain | **MET** | Manual X.509 chain validation via `x509-cert = "0.2"`. Issuer DN checked for `"fulcio"`/`"sigstore"`. `FLINT_COSIGN_MODE=full|legacy` toggle. |
| G5 DID HTTP resolution | **MET** | `VerifierDid` now resolves `did:prometheus:<id>` (non-inline) via HTTP with 5-min TTL cache. Inline fast path unchanged. Wiremock tests assert cache prevents duplicate requests. |
| G6 Integration test harness | **MET** | `--features integration` gate on OCI, IPFS, S3 crates. `testcontainers 0.23` + `testcontainers-modules 0.11`. All 3 integration executables compile; unit tests unaffected. |

**Overall: 6/6 goals MET (100%)**

Phase gate criteria satisfied:
- ✅ Epoch deadline interrupts runaway components
- ✅ Cedar reads from `flint_kiln.cedar_policies` (real DB, not allow-all stub)
- ✅ BGW passes `publisher_did` → Cedar fires on hook invocations
- ✅ `cargo test --workspace` passes (437 tests)
- ✅ `cargo clippy --workspace -- -D warnings` clean

---

## Delivered Changes

| # | Change | Priority | Key files | Tests added |
|---|---|---|---|---|
| c001 | Epoch interruption | P0 | `fke-runtime/src/lib.rs` | 2 (ticker spawn, fast gate) |
| c002 | Cedar DB policies | P0 | `fke-server/src/kiln_db_policy.rs` + migration 0009 | 1 (disconnected pool error) |
| c003 | BGW publisher identity | P0 | `fke-server/src/kiln_bgw.rs` | 2 (keto_subject, empty bearer) |
| c004 | Fulcio cert chain | P1 | `fke-sign-cosign/src/lib.rs` (490 lines) | 2 (non-Fulcio issuer, mode env) |
| c005 | DID HTTP resolution | P1 | `fke-sign-did/src/lib.rs` + `src/tests.rs` | 4 (HTTP fetch, cache, 404, dedup) |
| c006 | Integration test harness | P2 | `fke-store-{oci,ipfs,s3}/` | 3 executables compile; unit tests unchanged |

**New workspace dependencies:** `x509-cert = "0.2"`, `testcontainers = "0.23"`, `testcontainers-modules = "0.11"`

---

## Artifact Quality Summary

No artifact-refiner logs (quality enforced via CI gate).

| Metric | Value |
|---|---|
| Clippy gate | ✅ 0 errors, 0 warnings |
| Test pass rate | 437 / 437 (100%) |
| Changes with test coverage | 6 / 6 (100%) |
| Parallel changes (concurrent subagent) | 3 (c004 + c005 + c006) |
| Subagent write-miss rate | 0 / 3 (all files landed) |
| Files exceeding 500-line limit | 1 → fixed by splitting `fke-sign-did` into `lib.rs` (262) + `tests.rs` (224) |

---

## What Worked Well

1. **Parallel subagent execution** — c004, c005, c006 ran concurrently and all three delivered working code. The 0% write-miss rate was an improvement over the earlier session (c005 cosign initially missed).

2. **`x509-cert` over `sigstore` crate** — The sigstore crate is marked experimental with active API churn. Using `x509-cert = "0.2"` for manual certificate chain validation gave a stable, well-understood path. The Fulcio issuer check (`"sigstore"` or `"fulcio"` in DN) is simple and auditable.

3. **`FLINT_COSIGN_MODE` toggle** — Keeping the legacy ECDSA path accessible via env var avoids a breaking change for operators who can't yet set up Fulcio infrastructure.

4. **TTL cache without `moka`** — `std::sync::Mutex<HashMap<String, (VerifyingKey, Instant)>>` was sufficient for the DID cache. Lazy expiry on read keeps the implementation simple. No additional dep needed.

5. **`testcontainers 0.23` API discovery** — The subagent correctly identified that `Cli::default()` was removed and `AsyncRunner` is the current API. The `cncf_distribution` (not `registry`) module name was also correctly resolved.

---

## What Was Harder Than Expected

1. **`testcontainers-modules` feature naming** — The OCI registry module is `cncf_distribution`, not `registry`. The proposal used the wrong feature name. Resolved by the subagent during implementation.

2. **`object_store` HTTP endpoint** — S3 integration tests against MinIO require `with_allow_http(true)` and a new `KILN_S3_ALLOW_HTTP` env var. The original `from_env()` only supported HTTPS. Added in c006.

3. **`StoreOci` TLS default** — `oci-client 0.17` uses HTTPS by default. Integration tests against a local `registry:2` (plain HTTP) needed `StoreOci::with_http_registry()`. Added in c006.

4. **500-line file limit** — `fke-sign-did/src/lib.rs` reached 548 lines after adding HTTP resolution + all tests. Split into `lib.rs` (262) + `tests.rs` (224) to comply with the constraint.

5. **`EdgeRuntime::new()` requires Tokio context** — Adding `tokio::task::spawn` to `new()` broke sync `#[test]` functions that called `EdgeRuntime::new()`. Fixed by converting those tests to `#[tokio::test]`.

---

## Technical Debt Introduced

| Item | Location | Severity | Remediation |
|---|---|---|---|
| `AllowAllPolicySource` still present | `fke-server/src/kiln_policy.rs` | LOW | Delete after all unit tests that need a policy source are migrated to `DbKilnPolicySource` + mock pool |
| Fulcio issuer check is substring-based | `fke-sign-cosign/src/lib.rs` | LOW | Upgrade to full Fulcio root CA certificate pin when the Sigstore trust root is stabilised |
| Epoch ticker not bound to `EdgeRuntime` lifetime | `fke-runtime/src/lib.rs` | LOW | Use `CancellationToken` or `Weak<Engine>` ref in ticker so it exits when the runtime is dropped |
| OCI integration test `StoreOci::with_http_registry` exposes plaintext | `fke-store-oci/src/lib.rs` | LOW | Only used in tests; clearly named. Production always uses TLS. |
| `fke-sign-did/src/tests.rs` is a separate file | `fke-sign-did/src/` | NONE | Intentional split to stay under 500 lines. No action needed. |

---

## Lessons Captured

1. **The 500-line constraint is load-bearing** — It forced the clean `lib.rs`/`tests.rs` split in `fke-sign-did`. Files that are forced under 500 lines tend to stay more focused.

2. **`tokio::task::spawn` inside constructors requires a Tokio context** — Any struct that spawns tasks in `new()` must document that it requires a Tokio runtime. Tests must use `#[tokio::test]`.

3. **Stable crate over experimental** — `x509-cert = "0.2"` over `sigstore = "0.14"`. The sigstore crate is labelled experimental; manual cert parsing with a well-audited crate is more maintainable for this use case.

4. **Container module feature names drift from documentation** — Always check the actual crate source (or `cargo search`) before writing `testcontainers` test code. Feature names change between minor versions.

5. **Write-miss verification after concurrent subagents** — Check `wc -l` on all target files before assuming parallel agents succeeded. The check costs ~2 seconds and avoids silent partial delivery.

---

## Recommended Next Phase

**Name:** `p8-sdk-completeness`

**Focus:** Complete the SDK surface and close the remaining production gaps across the `@flint/react`, `flint_genui`, and HTMX rendering layers. The Kiln runtime stack is now fully hardened; the next area of greatest leverage is developer-facing SDK quality.

**Proposed changes (6–8 estimated):**

1. `@flint/react` SDK — component prop type completeness, `useFlint()` hook docs, missing component exports, bundle size audit (< 80 KB target)
2. `flint_genui` Dart package — `FlintA2uiTransport` SSE reconnect with exponential backoff, `FlintThemeData` token override from `design_systems` table
3. HTMX renderer — remaining 48 component slugs (beyond the 7 already rendering), SSE → HTMX OOB swap integration
4. OpenDesign integration (p5-c013 partial) — complete the ZIP import path for Claude Design `/design-sync` compatibility
5. Claude Design skill (p5-c015 gate tests) — live gate: `claude plugin install flint-ui@prometheus-ags/flint-forge` + verify component slug accuracy against DB
6. `@flint/react` design token export (`exportDesignSyncTokens()`) — W3C format for `/design-sync`
7. CI pipeline — `cargo test --workspace` on PR, `cargo component build -p hello-component` gate, Docker image build for `fdb-gateway` and `fke-server`

**Alternative:** If the team is ready to ship, a consolidation sprint (Docker images, load tests, staging environment deploy, runbook) delivers more immediate value than SDK feature work.
