# Kiln ABI Reference — `flint:host@0.1.0`

This document is the canonical reference for skill-component authors targeting
the Flint Kiln edge runtime. It covers the stable world interface, capability
contracts, security model, resource limits, and versioning policy.

`FLINT_KILN_ABI_VERSION=1`

---

## 1. World Overview

Every deployed skill component targets the **`flint:host@0.1.0`** WIT world
(`wit/flint/host/world.wit`).

A *skill component* is a WASM Component Model binary that:

1. **Exports** `wasi:http/incoming-handler@0.2.12` — the Kiln runtime calls
   `handle()` for every inbound HTTP invocation.
2. **Imports** one or more governed host capabilities (`db`, `llm`, `kv`,
   `identity`, `secrets`) declared in the component's signed manifest.

### Targeting in `Cargo.toml` / `component.toml`

```toml
# Cargo.toml (cargo-component ≥ 0.21)
[package.metadata.component]
package = "flint:host"
version  = "0.1.0"
```

Or via a `component.toml` at the workspace root:

```toml
[component]
package = "flint:host@0.1.0"
wit     = "wit/"
```

The Kiln host verifies the component's signed `FunctionManifest` — which
declares publisher DID, content digest, and requested capabilities — before
instantiation. Only the intersection of declared capabilities and Cedar grants
is made available to the component.

---

## 2. `wasi:http/incoming-handler` Contract

The host constructs a `KilnRequest` (method, URI, headers, body) and invokes
`handle()` on the component via the `wasi:http/incoming-handler@0.2.12` export.

**Request shape**:

| Field | Type | Description |
|-------|------|-------------|
| `method` | `String` | HTTP verb (`"GET"`, `"POST"`, …) |
| `uri` | `String` | Path + query (`"/functions/v1/my-skill?k=v"`) |
| `headers` | `Vec<(String, String)>` | Header name/value pairs |
| `body` | `Vec<u8>` | Raw request body |

**Response shape**:

| Field | Type | Description |
|-------|------|-------------|
| `status` | `u16` | HTTP status code |
| `body` | `Vec<u8>` | Raw response body |

> **Note**: Only `wasi:http` is exposed. Components have no raw TCP socket
> access. Outbound HTTP (via `wasi:http/outgoing-handler@0.2.12`) is available
> only when `HttpOutgoing` is declared in the signed manifest and Cedar permits
> it.

---

## 3. `db` Interface — Governed Database Access

```wit
@since(version = 0.1.0)
interface db {
    record host-error { code: string, message: string }
    query: func(sql: string, params: list<string>) -> result<list<string>, host-error>;
}
```

### Parameter Encoding

SQL uses positional placeholders: `$1`, `$2`, … Each element of `params` is a
**JSON-encoded value**:

| Rust type | `params` element | SQL binding |
|-----------|-----------------|-------------|
| `String`  | `"\"hello\""` | `'hello'` |
| `i64`     | `"42"` | `42` |
| `bool`    | `"true"` | `true` |
| `None`    | `"null"` | `NULL` |

Example:

```rust
kv_host.query(
    "SELECT id, name FROM users WHERE role = $1 AND active = $2",
    &[r#""admin""#.into(), "true".into()],
)
```

### Row Encoding

Each row in the result `list<string>` is a **JSON-encoded object** keyed by
column name: `{"id":1,"name":"Alice","active":true}`.

### `host-error` Codes

| `code` | Meaning |
|--------|---------|
| `CEDAR_DENY` | Cedar policy denied the query |
| `SQL_ERROR` | Database returned an error (message contains detail) |
| `TIMEOUT` | Query exceeded the per-invocation time budget |
| `UNSUPPORTED` | Operation not permitted (DDL, multi-statement, etc.) |

### Cedar Requirement

The publisher DID must hold Cedar grants for both `flint_kiln:invoke` **and**
`flint_db:query` before the host executes any SQL.

---

## 4. `llm` Interface — Governed Inference

```wit
@since(version = 0.1.0)
interface llm {
    record host-error { code: string, message: string }
    embed:    func(input: string, model: option<string>) -> result<list<f32>, host-error>;
    complete: func(prompt: string, opts: string)         -> result<string, host-error>;
}
```

### `embed(input, model)`

Returns an embedding vector of `f32` values. When `model` is `None` the host
uses the site-configured default embedding model.

### `complete(prompt, opts)`

`opts` is a **JSON-encoded** options object:

```json
{
  "model":       "gpt-4o-mini",
  "temperature": 0.7,
  "max_tokens":  512
}
```

All fields are optional; the host applies provider-level defaults for missing
keys. Returns the completion string on success.

### Routing

Requests are dispatched along **flint-gate → UAR → provider**. Provider API
keys are never exposed inside WASM linear memory; the host injects credentials
via Flint Vault at the boundary. The component identity visible to the provider
is the publisher DID, not the component's internal state.

---

## 5. `kv` Interface — Ephemeral Key-Value Store

```wit
@since(version = 0.1.0)
interface kv {
    get: func(k: string) -> option<list<u8>>;
    set: func(k: string, v: list<u8>);
}
```

- **Lifetime**: per-invocation only — discarded after `handle()` returns.
- **Persistence**: use `flint:db` for durable state across invocations.
- **Value type**: raw bytes (`list<u8>`); callers may encode as UTF-8 / JSON.
- **No Cedar gate**: `kv` operates in component-private memory managed by the
  host allocator; no authorization check is applied.

---

## 6. `identity` Interface — Origin JWT & Claims

```wit
@since(version = 0.1.0)
interface identity {
    origin-jwt: func() -> option<string>;
    claims:     func() -> string;
}
```

Both functions reflect the verified origin JWT injected by Kiln before
instantiation.

| Function | Return | Notes |
|----------|--------|-------|
| `origin-jwt()` | `option<string>` | Raw JWT string; `None` for system (BGW) invocations |
| `claims()` | `string` | JSON-encoded claim set, e.g. `{"sub":"did:key:z…","iss":"…","exp":…}` |

Neither function returns signing-key material or the secret used to verify the
token.

---

## 7. `secrets` Interface — Cedar-Gated Secret Access

```wit
@since(version = 0.1.0)
interface secrets {
    record host-error { code: string, message: string }
    resource secret {
        reveal: func() -> result<string, host-error>;
    }
    get: func(name: string) -> result<secret, host-error>;
}
```

### Two-Step Access Pattern

1. `get(name)` — returns an opaque `secret` resource handle if Cedar allows.
   The raw value is **not** returned at this step.
2. `reveal()` on the handle — returns the plaintext value.

### Security Guarantees

- **Cedar-gated**: the publisher must hold an explicit `flint_vault:reveal`
  grant; no grant → `reveal()` returns `host-error{code:"CEDAR_DENY"}`.
- **Audited**: every `reveal()` call is appended to `vault.access_log`.
- **Default-deny**: if the publisher DID has no matching grant, the call fails.
- **Host-brokered high-value secrets**: for secrets that only need to be
  forwarded to outbound calls (API keys, bearer tokens), the host injects them
  directly at the boundary so the plaintext never enters WASM linear memory.
  Use `reveal()` only when the component must inspect the value inline.

---

## 8. Fuel Limit

The Kiln runtime grants each invocation a fixed **fuel budget** measured in
Wasmtime fuel units (approximately one unit per WASM instruction executed).

| Parameter | Default | Override |
|-----------|---------|---------|
| Fuel per invocation | `10_000_000` (~10 M instructions) | `EdgeRuntime::with_fuel(n)` |

When the fuel budget is exhausted the component **traps** with `FuelExhausted`.
The host catches this trap and returns an HTTP 500 to the caller. The component
is not re-entered; any partial response is discarded.

---

## 9. Epoch Interruption

In addition to fuel, Kiln uses **epoch-based interruption** as a wall-clock
safety net for components that spend execution time in host calls (where fuel
is not consumed).

| Parameter | Default | Override |
|-----------|---------|---------|
| Ticker interval | `10 ms` | `KILN_EPOCH_INTERVAL_MS` env var |
| Epoch deadline | `1` | Not configurable via public API |

A background async task increments the Wasmtime engine epoch every interval.
When the component's epoch counter reaches the deadline (`1`) within a single
`handle()` invocation, the component **traps**. Set `KILN_EPOCH_INTERVAL_MS=0`
to disable epoch interruption (useful in tests that rely purely on fuel limits).

---

## 10. Cedar Authorization Decision Flow

```
Publisher identity
  └─ DID/Ed25519 signature  OR  Cosign/Sigstore signature on FunctionManifest
        │
        ▼
  FunctionManifest.publisher_did
        │
        ▼
  Pep::check(caller, flint_kiln:invoke)  ←── flint_kiln.cedar_policies (Postgres)
        │
    ┌───┴───┐
  Allow   Deny ──► bail!("Cedar policy denied kiln:invoke")
    │
    ▼
  Capability gate: manifest.capabilities ∩ Cedar-granted capabilities
        │
        ▼
  Instantiate component + dispatch wasi:http/incoming-handler
```

Key points:

- **`caller = None`** (BGW / system-level invocations): the Cedar gate is
  skipped entirely. Only trusted internal callers pass `None`.
- Cedar policies are stored in `flint_kiln.cedar_policies` and are consulted
  synchronously via the `Pep` trait injected into `EdgeRuntime::with_pep()`.
- **Default-deny**: a component whose publisher DID has no matching Cedar policy
  cannot exercise any host capability even if declared in its manifest.
- Each host capability (`db`, `llm`, `secrets`) performs an additional
  fine-grained Cedar check at the point of use (e.g. `flint_db:query`,
  `flint_vault:reveal`).

---

## 11. ContentId Format

Content addresses use the convention:

```
sha256:<64-char-lowercase-hex>
```

Example:

```
sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
```

IPFS CIDs (CIDv1, base32) are also accepted by the `ContentId` domain type but
the canonical form for newly published components is the `sha256:` prefix.
Component identity in Cedar policies references this digest.

---

## 12. Supported Store Backends

The Kiln component store can be backed by any of:

| Backend | Required env vars |
|---------|------------------|
| **Filesystem** | None — always available as fallback |
| **S3 / MinIO** | `AWS_ENDPOINT_URL`, `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_S3_BUCKET` |
| **OCI registry** | `FLINT_OCI_REGISTRY`, `FLINT_OCI_USERNAME`, `FLINT_OCI_PASSWORD` |
| **IPFS (Kubo)** | `FLINT_IPFS_URL` (default `http://localhost:5001`) |

Backend selection is determined at `EdgeRuntime` startup by the presence of the
relevant env vars. Filesystem is always available as a fallback; other backends
take precedence when their configuration is complete.

---

## 13. Versioning Policy

`FLINT_KILN_ABI_VERSION=1`

All interfaces in `flint:host@0.1.0` are **stable** as of this release:

- **Additive changes** (new optional functions, new record fields with defaults,
  new `host-error` codes) may land in a `0.1.x` patch release without prior
  announcement.
- **Breaking changes** (removing functions, altering existing signatures,
  removing error codes) require a **minor version bump** (`0.2.0`) and will be
  announced in this document at least one release cycle in advance.
- The WIT package version in `Cargo.toml` / `component.toml` is the contract
  anchor. Pin to `flint:host@0.1.0` to opt out of future breaking changes.

### Breaking Change Process

1. New interface lands in `flint:host@0.2.0` world alongside the old one.
2. `flint:host@0.1.0` world is maintained for one full release cycle.
3. Deprecation notice is added to this file with a migration guide.
4. `0.1.0` world is removed in the subsequent minor release.

For questions or to propose additions, open an issue referencing this document
and `FLINT_KILN_ABI_VERSION`.
