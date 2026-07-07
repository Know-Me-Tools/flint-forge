# p11-c002 — Kiln ABI Freeze

**Phase:** 11 — API Stability  **Priority:** P0  **Depends on:** none

## Problem

`wit/flint/host/world.wit` defines the `edge-function` world at `0.1.0` but
has no `@since` annotations on individual interfaces and no `docs/api/kiln-abi.md`
skill-author reference. Skill authors have no machine-readable or human-readable
contract specifying which interfaces are stable.

## WIT `@since` annotation plan

Add `@since(version = 0.1.0)` to each interface declaration in
`wit/flint/host/world.wit`:

```wit
/// Governed DB access — routes through flint-gate under the origin JWT.
@since(version = 0.1.0)
interface db { … }

@since(version = 0.1.0)
interface llm { … }

@since(version = 0.1.0)
interface kv { … }

@since(version = 0.1.0)
interface identity { … }

@since(version = 0.1.0)
interface secrets { … }
```

`@since` is informational in `cargo component 0.21.1` — it does not enforce
semver gates at compile time but provides machine-readable stability metadata
for documentation generators and future tooling.

Also add a `stability:` comment block to the world declaration:

```wit
/// The edge-function world: every deployed WASM component targets this world.
///
/// ## Stability
///
/// All interfaces in this world are **stable** as of `flint:host@0.1.0`.
/// Breaking changes will increment the package minor version and be announced
/// in `docs/api/kiln-abi.md`.
world edge-function { … }
```

## `docs/api/kiln-abi.md` content outline

Skill-author reference (~200 lines) covering:

1. **World overview** — `flint:host@0.1.0`; how to target it in a component
2. **`wasi:http/incoming-handler` contract** — request format, response format,
   status codes
3. **`db` interface** — `query` signature, param encoding, row encoding, error codes
4. **`llm` interface** — `embed` and `complete` signatures, options object shape
5. **`kv` interface** — get/set semantics, lifetime (per-invocation, non-durable)
6. **`identity` interface** — `origin-jwt` and `claims` semantics
7. **`secrets` interface** — `get`/`reveal` flow, Cedar grant requirement, audit trail
8. **Fuel limit** — default 10 M instructions; behaviour on exhaustion
9. **Epoch interruption** — 10 ms ticker; trap on epoch deadline 1
10. **Cedar authz flow** — publisher identity → Cedar decision → deny-by-default
11. **ContentId format** — `sha256:<hex>` convention
12. **Supported store backends** — fs, S3/MinIO, OCI, IPFS
13. **`FLINT_KILN_ABI_VERSION`** versioning policy
