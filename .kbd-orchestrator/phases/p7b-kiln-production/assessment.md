# Assessment — p7b-kiln-production

**Phase:** p7b-kiln-production
**Assessed:** 2026-07-04
**Previous phase:** p6b-kiln-hardening (8/8 done; clippy clean; 426 tests passing)

---

## Codebase Inventory

### What p6b delivered (foundation for p7b)

| Crate | State |
|---|---|
| `fke-runtime` | Real wasmtime 26 engine, `ProxyPre` cache, `WasiHttpView`, Cedar gate, fuel limit |
| `fke-server` | `/functions/v1` invoke, `/admin/functions`, Kiln BGW, `AllowAllPolicySource` |
| `fke-sign-did` | Ed25519 + inline `did:prometheus` key; **no HTTP resolution** |
| `fke-sign-cosign` | Rekor fetch + ECDSA P-256 verify; **no Fulcio chain** |
| `fke-store-oci/ipfs/s3` | All three methods implemented; **no integration tests** |

---

## Gap Analysis by Goal

### G1 — Epoch Interruption (P0)

**Current state:** `fke-runtime` uses only fuel (`store.set_fuel(DEFAULT_FUEL)`). `Config::epoch_interruption(true)` is not set. No background epoch ticker exists.

**Gap:**
- `Config::epoch_interruption(true)` needs to be added to the `EdgeRuntime::new()` config block
- `Engine::increment_epoch()` must be called periodically by a background task (tokio interval, ~10 ms)
- `Store::set_epoch_deadline(1)` must be set per invocation so the store traps when the epoch ticks past the deadline
- The `EdgeRuntime` needs to hold an `Arc<Engine>` (already does) so the background task can call `engine.increment_epoch()`

**Work required:**
- `EdgeRuntime::new()`: add `cfg.epoch_interruption(true)`
- `EdgeRuntime::new()`: spawn `tokio::task::spawn` that sleeps 10 ms then calls `self.engine.increment_epoch()`; store `JoinHandle` in a field `_epoch_ticker: JoinHandle<()>`
- `EdgeRuntime::handle()`: add `store.set_epoch_deadline(1)` after `store.set_fuel()`

**Effort:** Small — 3 targeted edits, no new deps. Pattern is well-documented in wasmtime docs.

**Risk:** The ticker continues even after `EdgeRuntime` is dropped (JoinHandle not aborted). Mitigate: wrap the engine in `Arc` + `WeakRef` in the ticker, or use `CancellationToken`.

---

### G2 — Cedar Policies from DB (P0)

**Current state:** `fke-server/src/kiln_policy.rs` contains `AllowAllPolicySource` which permits every `kiln:invoke` regardless of principal. This is explicitly a bootstrap placeholder.

**Gap:**
- No `flint_kiln.cedar_policies` table exists (migration needed)
- No `DbPolicySource` in `fke-server` (exists in `fdb-gateway/src/policy_source.rs` as a reference implementation)
- `fke-server/src/main.rs` wires `AllowAllPolicySource`; must switch to `DbKilnPolicySource`

**Work required:**
- Create `migrations/0009_flint_kiln_cedar_policies.sql` — same schema as `flint_meta.cedar_policies`
- Create `crates/fke-server/src/kiln_db_policy.rs` — `DbKilnPolicySource` that loads from `flint_kiln.cedar_policies` (copy structure from `fdb-gateway/src/policy_source.rs`, change table name)
- Update `fke-server/src/main.rs` to use `DbKilnPolicySource::new(pool.clone())`
- Seed a default `permit(principal, action, resource)` row in `migrations/0009` so the system works before any policies are authored

**Effort:** Small-medium — direct port of existing code. The Kiln Cedar policy namespace (`kiln:invoke`) is already in `forge-policy/src/kiln.rs`.

---

### G3 — BGW Publisher Identity (P0)

**Current state:** `fke-server/src/kiln_bgw.rs:134` passes `None` as `caller` to `EdgeRuntime::handle()`:

```rust
None, // BGW = system caller; Cedar gate is skipped
```

This means every hook-triggered invocation bypasses Cedar entirely.

**Gap:**
- The `FunctionManifest.publisher_did` is available after registry lookup
- Need to synthesize `RlsContext { role: "kiln_publisher", keto_subject: publisher_did, claims_json: ... }` and pass it as `Some(&publisher_rls)` to `handle()`

**Work required:**
- Add `fn publisher_rls(manifest: &FunctionManifest) -> RlsContext` helper in `kiln_bgw.rs`
- Change `invoke_function()` to call `let rls = publisher_rls(&manifest);` and pass `Some(&rls)` to both `handle()` calls

**Effort:** Trivial — ~10 lines in `kiln_bgw.rs`. No new deps.

**Note:** `forge-identity` is already a dep of `fke-runtime`. Need to check if it's also a dep of `fke-server` (it is, transitively via `fdb-auth`). May need to add direct dep.

---

### G4 — Fulcio Certificate Chain (P1)

**Current state:** `fke-sign-cosign` fetches the Rekor log entry, extracts a PEM public key, and verifies ECDSA P-256 directly. The Fulcio certificate chain (leaf cert → intermediate → Fulcio root) is not validated. The public key is trusted at face value from the Rekor response.

**Gap:**
- `sigstore = "0.14.0"` is not a workspace dep
- No `sigstore::cosign::CosignCapabilities` integration
- The current `VerifierCosign::verify()` trusts the PEM from Rekor without checking certificate validity or chain

**Work required:**
- Add `sigstore = "0.14"` to `[workspace.dependencies]`
- Add it to `fke-sign-cosign/Cargo.toml`
- Replace or augment the existing `verify()` with `sigstore::cosign` verification flow

**Effort:** High — `sigstore 0.14` API has significant churn. Need to audit API before implementation. The `sigstore-verify` crate (`0.10.0`) may be a simpler alternative.

**Risk:** `sigstore` crate marks itself as "experimental". Recommend pinning to exact version and reading the changelog before adding. The existing ECDSA verification remains functional; Fulcio chain adds defense-in-depth, not a correctness fix.

---

### G5 — DID HTTP Resolution (P1)

**Current state:** `fke-sign-did::parse_did()` only handles the inline `did:prometheus:<base64url-pubkey>` format. If the DID string does not embed a key (i.e., it is a reference DID like `did:prometheus:0x1234abc`), `parse_did()` returns `SignError::Invalid`.

**Gap:**
- No HTTP client code in `fke-sign-did`
- No `FLINT_DID_RESOLVER_URL` env var reading
- No TTL cache for resolved keys
- `reqwest` is a workspace dep but not yet in `fke-sign-did/Cargo.toml`

**Work required:**
- Extend `parse_did()`: if the base64url-decode of the DID suffix produces < 32 bytes OR isn't a valid Ed25519 key, try HTTP resolution
- Add `GET {FLINT_DID_RESOLVER_URL}/v1/did/{did}` → parse `{ "verificationMethod": [{ "publicKeyBase64Url": "..." }] }`
- Add a simple TTL cache: `std::sync::Mutex<HashMap<String, (VerifyingKey, std::time::Instant)>>` inside `VerifierDid` with 5-min TTL (avoids `moka` dep for now)

**Effort:** Medium — HTTP call + response parsing + cache struct. New `reqwest` dep in `fke-sign-did/Cargo.toml`. The inline path stays fast; HTTP only for resolution fallback.

---

### G6 — Integration Test Harness (P2)

**Current state:** `fke-store-oci`, `fke-store-ipfs`, `fke-store-s3` each have 2 tests marked `#[ignore]` or no live-infra tests at all. `testcontainers` is not a workspace dep.

**Gap:**
- No `--features integration` feature gate in any store crate
- `testcontainers = "0.23"` not in workspace
- No `docker-compose.test.yml` or similar for local test infra

**Work required:**
- Add `testcontainers = "0.23"` + `testcontainers-modules` to workspace dev-deps
- Add `[features] integration = []` to `fke-store-oci/ipfs/s3/Cargo.toml`
- Write `#[cfg(feature = "integration")]` tests using `testcontainers::images::generic::GenericImage` for `registry:2`, MinIO, and Kubo

**Effort:** Medium — boilerplate per crate. The actual test logic is simple (put/get/exists round-trip). Main complexity is `testcontainers` API familiarity.

---

## Dependency Map

```
G1 (epoch)          — independent, no new deps
G2 (Cedar DB)       — independent; migration needed first
G3 (BGW identity)   — depends on G2 (Cedar must be real before BGW identity matters)
G4 (Fulcio chain)   — independent; needs sigstore dep audit first
G5 (DID HTTP)       — independent; needs reqwest in fke-sign-did
G6 (integration)    — independent; lowest priority
```

Optimal order: **G1 + G5 in parallel → G2 → G3** (G2 must precede G3 to be meaningful); G4 and G6 independently.

---

## Risk Register

| Risk | Likelihood | Severity | Mitigation |
|---|---|---|---|
| `sigstore 0.14` API too unstable | MEDIUM | MEDIUM | Audit API first; consider `sigstore-verify = "0.10"` alternative |
| Epoch ticker not cancelled on `EdgeRuntime` drop | LOW | LOW | Wrap engine ref in `Arc` + `Weak` in ticker; exit loop when upgrade fails |
| `kiln_bgw` Cedar fire on publisher_did changes Cedar semantics | LOW | MEDIUM | The default `AllowAllPolicySource` permits all anyway; no regression until real policies are loaded |
| `testcontainers` Docker-not-available in CI | MEDIUM | LOW | Gate with `#[cfg(feature = "integration")]` + env-based skip |

---

## Assessment Summary

| Goal | Gap Size | Effort | Blocking? |
|---|---|---|---|
| G1 Epoch interruption | Small — 3 edits in `lib.rs` | Low | No |
| G2 Cedar from DB | Small-Med — migration + port of existing code | Low-Med | G3 is meaningless without it |
| G3 BGW publisher identity | Trivial — ~10 lines in `kiln_bgw.rs` | Trivial | No (but do after G2) |
| G4 Fulcio chain | Large — `sigstore` crate audit + integration | High | No |
| G5 DID HTTP resolution | Medium — HTTP + cache struct in `fke-sign-did` | Medium | No |
| G6 Integration tests | Medium — `testcontainers` boilerplate × 3 crates | Medium | No |

**No external blockers.** All 6 changes are implementable against the current codebase.

**Handoff to plan:** Start with G1 (trivial, high value) and G3 (trivial, closes Cedar bypass). Then G2 (makes G3 meaningful). G4/G5/G6 can be parallel after the P0 triad.
