# Assessment — p0-workspace-foundation

**Date:** 2026-06-29
**Sycophancy posture:** corrected — this report names what is incomplete, divergent, or missing, not what looks good.

---

## Phase 0 Gate Status: PARTIALLY GREEN

| Change | Status | Gate condition |
|---|---|---|
| c001 — Workspace foundation | ✅ GATE PASSED | `cargo check` + clippy::pedantic + fmt all green (toolchain 1.90) |
| c002 — Postgres 18 image | ✅ GATE PASSED (with deferred items) | Image built, extensions verified; **pg_graphql deferred to Phase 3** |
| c003 — WIT contract freeze | ❌ GATE NOT MET | WIT file exists but tasks are unchecked; `wasm-tools` not installed locally; sample component not built |
| c004 — fabric WatchEntityType | ❌ GATE NOT MET | Cross-repo; all tasks unchecked; no evidence of work started |

**Phase 0 cannot be called complete.** c003 and c004 remain open. c002 passed with a significant deferral (pg_graphql) that will block Phase 3 entry.

---

## What Is Actually Implemented

### Genuinely working (not stubs)

**forge-domain** — real types: `TenantId`, `SubjectId`, `Json` (alias), `ForgeError`.

**forge-identity** — real structs: `Claims`, `RlsContext` with `subject()`/`tenant()` helpers, `outbound_headers()` (Option-3 headers). `verify_and_build()` is a `todo!()` — the JWKS verification is explicitly deferred to p2-c001.

**forge-policy** — `Decision`, `Request`, `Pep` trait. Trait body is defined but no Cedar integration exists.

**fdb-domain** — `TableMeta`, `RestQuery`, `RestResult`, `ChangeEvent`, `ChangeOp`, `SubscriptionSpec`, `RlsContext` re-export, `GraphQlRequest`. All real types.

**fdb-ports** — all five traits declared: `DatabaseBackend`, `SchemaProvider`, `RestExecutor`, `GraphQlExecutor`, `ChangeStreamSource`. These are signatures only; no implementations.

**fdb-app** — `Quarry` struct wiring three `Arc<dyn Trait>` ports. No use-case logic — just the composition skeleton.

**fdb-postgres** — three structs (`PgBackend`, `PgGraphQl`, `PgRest`) each implementing one port. All three method bodies are `todo!()`.

**fdb-auth** — stub struct, no implementation.

**fdb-realtime** — stub struct, no implementation.

**fdb-gateway** — Axum server with `/healthz` route. That is the only working route.

**fke-domain** — `FunctionManifest`, `ContentId`, `Capability`, `CompilationStrategy`, `TargetArch`, `ComponentError`. Real types.

**fke-ports** — `ComponentStore`, `SignatureVerifier`, `ComponentRegistry`, `Compiler` traits. Signatures only.

**fke-runtime** — stub struct, no Wasmtime integration.

**fke-store-{oci,ipfs,s3,fs}** — all four adapters: each has a struct and `impl ComponentStore` with all three methods as `todo!()`.

**fke-sign-{did,cosign}** — both have a struct and `impl SignatureVerifier` with `verify()` as `todo!()`.

**fke-registry** — one method as `todo!()`.

**fke-server** — Axum server with `/healthz`. That is the only working route.

**forge-cli** — prints "forge-cli". Nothing else.

**ext-flint-vault** — 489 lines. This is the most advanced crate in the repo: full XChaCha20-Poly1305 AEAD crypto, HKDF-SHA256 key derivation, `secrecy::Secret` DEK, `vault.secrets` and `vault.access_log` schema, `create_secret`/`get_secret`/`resolve_api_key` SECURITY DEFINER functions, audit trail. **This is real implementation**, not scaffold. KMS unwrap integration is marked as a §8 open item.

**ext-flint-auth** — SQL-only (installed via `.sql` extension file in the Docker image). The `auth.*` SQL functions (`auth.jwt()`, `auth.uid()`, `auth.role()`, `auth.bearer()`) are implemented and verified running in the container. The Rust crate is a stub.

**ext-flint-hooks** — SQL-only extension installed. `flint.webhooks`, `flint.webhook_outbox`, `flint.dispatch_webhook()` DDL exists in the extension SQL. Runtime dispatch logic is in SQL/pg_net. The Rust crate is a stub.

**ext-flint-llm** — pgrx crate compiled into the Docker image. `llm_version()` function verified. Background worker is a future phase item.

**Postgres 18 image** — multi-stage Dockerfile builds and runs. Extensions verified: `flint_auth 0.1.0`, `flint_hooks 0.1.0`, `flint_llm 0.1.0`, `pg_net 0.20.3`, `pgcrypto 1.4`, `vector 0.8.3`. `wal_level=logical`, `shared_preload_libraries=pg_net`. **pg_graphql absent** (no PG18 release; supabase/pg_graphql#614).

**WIT contract** — `wit/flint/host/world.wit` exists with all six interfaces and `edge-function` world. WASI 0.2.0 pinned. **However:** `params: list<string>` in `db.query` diverges from the spec's `list<json>`, and `secrets.get` returns `result<string, string>` rather than a `resource secret { reveal }` as spec §5.4 requires. The `wasm-tools` validation task is unchecked; no sample component built.

---

## Gap Analysis: Phase 0 → Full Implementation

### What "full implementation" means per RFC-FORGE-001

The spec defines 7 phases + hardening (Phase 7) as the complete implementation. The current state is approximately Phase 0 scaffold + partial c001/c002 completion with c003/c004 open.

### Gaps by subsystem

#### Flint Quarry (Phases 1–3; ~0% functional)

- `verify_and_build()` in `forge-identity` is a stub — no JWKS fetch, no JWT signature verification, no `RlsContext` assembly from a real token.
- `PgBackend::acquire()` is a stub — no deadpool-postgres pool, no `SET LOCAL` transaction injection.
- `SchemaProvider` has no postgres implementation — no `introspect()`, no DDL watch receiver.
- `PgRest::execute()` is a stub — no PostgREST-compatible query builder, no filter operator parsing, no `Range`/`Content-Range` headers, no `Prefer` header handling.
- `PgGraphQl::execute()` is a stub — no `graphql.resolve()` passthrough. Additionally blocked by pg_graphql not being in the image until Phase 3.
- `fdb-realtime` is a stub — no gRPC tonic client, no `WatchEntityType` consumption, no Keto coarse gate, no per-event RLS re-query.
- `SchemaRegistry` / `ArcSwap` hot-reload pattern not implemented.
- No Axum routes beyond `/healthz` in `fdb-gateway`.
- No connection pool wiring in the composition root.

#### Flint Anvil (Phases 1 + partial in image; ~25% functional)

- `flint_auth` SQL functions are live in the image — this is working.
- `flint_hooks` DDL is in the image. The `dispatch_webhook()` SECURITY DEFINER function exists in SQL. What is not done: the **durable tier dispatcher** (Iggy BGW / `FOR UPDATE SKIP LOCKED` consumer), Option-3 outbound service-token injection at runtime (needs flint-gate service credential from Vault), and gateway admin API to provision triggers dynamically.
- `flint_llm` compiles and installs. Background worker, `llm.jobs` queue, `llm.enable_embedding` declarative surface, and Surface 1 sync functions are all deferred to Phase 4.
- `flint_vault` has real crypto code (489 lines) but: KMS unwrap (KEK provider) is open, the `_PG_init` bootstrap is unfinished, `PGDATAKEYUNWRAPCMD`-style integration is not wired, the `access_log` trigger is not confirmed complete, and the v1 "SECURITY DEFINER SQL path" residual risk is acknowledged but not resolved.

#### Flint Kiln (Phases 5–6; ~0% functional)

- `fke-runtime` is a stub — no Wasmtime engine, no `ProxyPre` cache, no epoch/fuel limits, no `StoreLimits`, no Cedar-gated `Linker`.
- All four `ComponentStore` adapters are `todo!()`.
- Both `SignatureVerifier` adapters are `todo!()`.
- `fke-registry` is a stub.
- No Axum routes beyond `/healthz` in `fke-server`.
- No AOT compilation pipeline, no `.cwasm` cache, no control/data-plane split build.
- No Cedar policy evaluation connected to anything.

#### WIT contract (c003 — open)

- `wit/flint/host/world.wit` exists but has two spec divergences (see above).
- `wasm-tools` not installed on the build path — validation task not run.
- No sample component built against the WIT — freeze is not confirmed.

#### fabric WatchEntityType (c004 — not started)

- This is a cross-repo change in `flint-realtime-fabric`. No evidence of work begun. It blocks Phase 3 subscriptions.

#### Forge CLI

- Placeholder only — no commands implemented (register functions, manage hooks, run migrations as specced in §6).

---

## What It Takes to Reach Full Implementation

### Phase 0 completion (remaining)

**c003 (blocking — do this first):**
1. Install `wasm-tools` and `wit-bindgen` on the dev/CI path.
2. Fix `db.query` params type: `list<string>` → `list<json>` (or `list<value>` — resolve the WIT JSON representation).
3. Fix `secrets` interface: implement `resource secret { reveal: func() -> result<string, error> }` and make `get` return `result<secret, error>`, not `result<string, string>`.
4. Run `wasm-tools component wit wit/flint/host/world.wit` to green.
5. Generate Rust bindings with `wit-bindgen`; build a trivial `hello` component to `wasm32-wasip2`.
6. Tag `flint:host@0.1.0` as frozen in docs.

**c004 (non-blocking for Phases 1–2; needed before Phase 3):**
- Requires working in `flint-realtime-fabric` repo. Proto change, Keto gate, integration test.

### Phase 1 — Flint Anvil (estimated 2–3 change sets)

- `p1-c001-flint-auth`: Wire `ext-flint-auth` Rust crate with pgrx so `auth.*` functions are installable via `cargo pgrx` (currently SQL-only). Write GUC contract integration tests via `pgrx_tests`.
- `p1-c002-flint-hooks-standard`: Implement the gateway admin API (Axum routes) for webhook registration. Wire `dispatch_webhook()` to read flint-gate service credential from Vault at runtime. Option-3 header construction. Standard tier (pg_net) end-to-end: INSERT → dispatch → signed delivery.
- `p1-c003-flint-hooks-durable`: Implement the outbox consumer (BGW or Iggy-backed), `FOR UPDATE SKIP LOCKED`, retry with backoff, ordering guarantee.

### Phase 2 — Flint Quarry REST (estimated 3 change sets)

- `p2-c001-fdb-auth`: Implement `verify_and_build()` in `forge-identity` — fetch JWKS from flint-gate issuer, verify RS256/ES256 signature, extract claims, build `RlsContext`. Requires the flint-gate contract to be pinned (§8 item 4).
- `p2-c002-fdb-postgres`: Wire deadpool-postgres pool in `PgBackend::acquire()`. Implement the three `SET LOCAL` statements. Implement `SchemaProvider::introspect()` (query `information_schema`) and `subscribe_ddl()` (poll or `flint_hooks` DDL trigger).
- `p2-c003-rest-executor`: Implement `PgRest::execute()` — PostgREST-compatible filter/select/order/limit/offset query builder, `Range`/`Content-Range`, `Prefer` header, `/rpc` function calls, pgvector `<->` similarity route.

### Phase 3 — Flint Quarry GraphQL (estimated 4 change sets; blocked on pg_graphql + c004)

- pg_graphql PG18 support is the long-pole external dependency. Options: (a) wait for upstream release, (b) build from a pinned SHA of master, (c) run GraphQL on PG17 sidecar.
- `p3-c001`: `PgGraphQl::execute()` → `graphql.resolve()` passthrough.
- `p3-c002`: `fdb-realtime` gRPC client + `WatchEntityType` + Keto gate + per-event RLS re-query + `graphql-transport-ws` subscription handler.
- `p3-c003`: introspection merge (pg_graphql SDL ∪ subscription SDL).
- `p3-c004`: opt-in predicate-pushdown.

### Phase 4 — Flint Ember (estimated 4 change sets)

- Wire `flint_llm` extension to route calls through flint-gate/UAR (not directly to providers). Resolve Vault credential access for the flint-gate auth token.
- Implement `llm.jobs` BGW: tokio runtime in a pgrx background worker, `FOR UPDATE SKIP LOCKED` dequeue, batching, rate-limit governor, write-back via SPI.
- Implement `llm.enable_embedding` / `llm.enable_summary` declarative layer.
- `llm.embed`/`llm.complete` synchronous surface with interrupt/cancellation safety (pgrx interrupt handling is a known engineering risk — budget time).

### Phase 5 — Flint Kiln runtime + invocation (estimated 3 change sets)

- Implement `fke-runtime`: Wasmtime engine config, `ProxyPre` cache keyed by component digest, per-request `Store` instantiation, epoch-based interruption, `StoreLimits`, Cedar-gated `Linker`.
- Implement `flint:db/llm/kv/identity/secrets` host capability impls routing through flint-gate.
- Wire `/functions/v1/<name>` invocation route in `fke-server` — compiler-off build feature.

### Phase 6 — Flint Kiln signing, storage, AOT (estimated 4 change sets)

- Implement `fke-sign-did` (Ed25519 + DID-VC) and `fke-sign-cosign` (Sigstore/OIDC) adapters.
- Implement all four `ComponentStore` adapters (OCI primary, IPFS, S3, fs).
- Implement AOT control plane: `Engine::precompile_component` → `.cwasm` cache keyed `(digest, target, wasmtime_version)`, cross-compile for `all-arch`.
- Admin REST routes: register/version/activate/delete.

### Phase 7 — Hardening

- Webhook→Kiln wiring; shared outbox/JWT capture across flint_hooks + flint_llm; Wasmtime host shared with UAR; backpressure contract with flint-gate; subscription cardinality limits; Keto cache invalidation; load tests; Dagger E2E pipeline; observability (tracing spans, metrics).

---

## Open External Dependencies (blocking at specific phases)

| Dependency | Blocks | Status |
|---|---|---|
| `pg_graphql` PG18 support (supabase/pg_graphql#614) | Phase 3 | No released build; track upstream or build from source |
| `flint-gate` service-identity credential + minted claim set (§8 #4) | Phase 2 (JWT verify), Phase 1 (Option-3 outbound) | Unresolved — pin exact claim shape before p2-c001 |
| `WatchEntityType` RPC in `flint-realtime-fabric` (c004) | Phase 3 subscriptions | Not started |
| Flint Vault KEK provider (§8 #6a) | `flint_vault` production use | Unresolved — Azure Key Vault via managed identity is the stated target |
| Wasmtime version / WASI 0.2 stability (§8 #3) | Phase 5 | Verify current `wasmtime` crate supports `wasi:http/proxy` Component Model; confirm before p5-c001 |
| UAR WASM host substrate (§2.4 convergence invariant #2) | Phase 5 (shared host) | Requires coordination with UAR team before `fke-runtime` is designed |

---

## Sycophancy-Corrected Summary

The scaffold is well-structured and the quality gates for c001 and c002 passed. That is real. However:

1. **The codebase is ~5–8% of the way to full implementation by functional surface area.** The workspace compiles and the Docker image boots — these are prerequisites, not product.
2. **Every user-visible capability — REST queries, GraphQL, subscriptions, edge functions, in-DB LLM, webhook dispatch — is a `todo!()`** at the Rust level or an unwired SQL stub.
3. **c003 is incomplete despite a WIT file existing.** The file has spec divergences (param types, `secrets` resource model), the validation toolchain is absent, and the freeze is not confirmed.
4. **The pg_graphql deferral in c002 is a real Phase 3 gate risk**, not a minor footnote. There is no PG18 release and the upstream issue is open. This needs a decision (build from source, PG17 sidecar, or wait) before Phase 3 planning begins.
5. **Flint Vault is the most complete subsystem** (~489 lines of real crypto code) but is not production-ready: KMS unwrap is unimplemented, `_PG_init` bootstrap is incomplete, and the residual SQL-round-trip plaintext risk is acknowledged but unresolved.
6. **Full implementation requires completing Phases 0–7** as specified. Based on the change-set counts in §7 (approximately 25–30 change sets across all phases), and the engineering density of each, this is a substantial build — measured in months of focused work, not weeks.

---

## Immediate Next Actions (in order)

1. **Close c003:** install `wasm-tools`, fix WIT spec divergences, validate, build sample component, declare freeze.
2. **Decide on c004 timeline:** schedule the cross-repo proto change in `flint-realtime-fabric` before Phase 3 planning — it is the longest external lead time item.
3. **Resolve the flint-gate contract (§8 #4):** pin minted claim shape and service-identity credential format before writing p2-c001. JWT verification cannot be implemented against an unspecified issuer.
4. **Decide pg_graphql strategy** (supabase/pg_graphql#614): build from source SHA, PG17 sidecar, or hold. Document the decision before Phase 3 kickoff.
5. **Begin Phase 1 (Flint Anvil):** start p1-c001-flint-auth, which has the lowest external dependency surface.
