# Reflection — p6-kiln-runtime

**Completed:** 2026-07-04
**Gate result:** MVP PASSED (4/4 changes, clippy clean, 387 workspace tests)

## What shipped

| Change | Deliverable |
|---|---|
| p6-c001 | `fke-runtime`: wasmtime 26 Engine + `InstancePre` cache + fuel limits + capability gate |
| p6-c002 | `fke-registry`: `PgRegistry` + `PgComponentStore` against `flint_kiln` Postgres schema |
| p6-c003 | `fke-server`: `/functions/v1/{name}` invoke endpoint + `/admin/functions` register/list |
| p6-c004 | `fke-store-fs`: content-addressed local artifact store with tokio async I/O |
| infra | Migration 0008 — `flint_kiln.functions` + `artifacts` + `invocations` tables |

## What worked well

- wasmtime 26 Component Model API is clean — `InstancePre<S>` cache pattern compiles first try.
- The `#![forbid(unsafe_code)]` constraint held: no unsafe deserialization needed because we use safe `Component::from_binary()` for now. Pre-compiled `.cwasm` deserialization can be added behind a feature flag when production latency demands it.
- `fke-store-fs` with content-addressed paths sharded by 2-char prefix maps cleanly onto the `ComponentStore` trait.
- The hook outbox `target_type='kiln'` stub (wired in p7-c001) is now backed by real infrastructure — the BGW can land without any schema changes.

## What was harder than expected

- `wasmtime-wasi` v26 no longer exports a separate `component-model` feature — it is built-in. Updated workspace dep accordingly.
- `Component::serialize()` / `Component::deserialize()` require `unsafe` — safe path deferred to p6b-c001 with `AotCompiler` behind `features = ["compiler"]`.
- `wasmtime-wasi-http` is a separate crate not yet wired — deferred to p6b after WIT bindings land.

## What was NOT built (deferred to p6b)

- **Cedar policy gate** on capability intersection before instantiation (`p6-c005`)
- **Kiln BGW** — drain `flint.webhook_outbox WHERE target_type='kiln'` (`p6-c006`)
- **WIT contract freeze** + `wasi:http/incoming-handler` binding generation (`p6-c007`)
- **Signature verifiers** — `fke-sign-cosign` and `fke-sign-did` still `todo!()` (`p6b-c004/c005`)
- **Remote stores** — `fke-store-oci`, `fke-store-ipfs`, `fke-store-s3` still `todo!()` (`p6b-c006/c007/c008`)

## Recommended Next Phase

**Name:** `p6b-kiln-hardening`

**Goal:** Complete the Kiln WASM edge runtime — security gates, remote artifact stores, WIT bindings, and the hook-to-Kiln BGW. By the end of p6b, the full path from `flint_hooks → webhook_outbox → Kiln BGW → fke-runtime → WASM function` is live end-to-end.

**Changes (8 planned):**

| Change ID | Title | Priority |
|---|---|---|
| p6b-c001 | Cedar capability gate in `fke-runtime` — intersection check before instantiation | P0 |
| p6b-c002 | Kiln BGW — drain `flint.webhook_outbox WHERE target_type='kiln'`, invoke `fke-runtime` | P0 |
| p6b-c003 | WIT contract: `flint:host@0.1.0` — freeze interface, generate host bindings | P0 |
| p6b-c004 | `fke-sign-did`: Ed25519 + `did:prometheus` VC verifier (sovereign default) | P1 |
| p6b-c005 | `fke-sign-cosign`: Sigstore/Cosign OCI registry verifier | P1 |
| p6b-c006 | `fke-store-oci`: OCI registry artifact store | P1 |
| p6b-c007 | `fke-store-ipfs`: IPFS artifact store (kubo HTTP API) | P2 |
| p6b-c008 | `fke-store-s3`: S3/R2 artifact store | P2 |

**Gate:** `fke-server` can receive a `target_type='kiln'` webhook payload, look up the function, verify its signature, load the WASM from a remote store, enforce Cedar capabilities, and return a valid HTTP response to the caller — all without `todo!()` panics.
