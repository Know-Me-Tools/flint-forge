# p10-c003 — Dependency CVE Remediation + `cargo audit` CI Gate

**Phase:** 10 — Production Launch
**Priority:** P0 — must ship first (unblocks all other changes)
**Depends on:** none

## Problem

`cargo audit` reports 5 CVSS ≥ 7.0 advisories including 2 CVSS-9.0 criticals.
The CI pipeline has no `cargo audit` step. There is no `.cargo/audit.toml` to
document justified allowlist entries.

### CVSS ≥ 7.0 blockers

| Advisory | Crate | CVSS | Root dep | Fix |
|---|---|---|---|---|
| RUSTSEC-2026-0096 | `wasmtime 26` | **9.0** | `fke-runtime`, `fke-server` | Upgrade `wasmtime 26 → 46` |
| RUSTSEC-2026-0095 | `wasmtime 26` | **9.0** | same | same |
| RUSTSEC-2026-0195 | `quick-xml 0.37.5` | 7.5 | `object_store 0.11` → `fke-store-s3` | Upgrade `object_store 0.11 → 0.14` |
| RUSTSEC-2026-0194 | `quick-xml 0.37.5` | 7.5 | same | same |
| RUSTSEC-2026-0149 | `fxhash` (via wasmtime) | 7.5 | transitively via wasmtime | Fixed by wasmtime upgrade |

## Solution

### 1. Upgrade `wasmtime 26 → 46`

`wasmtime 46.0.1` is the current latest and fixes all wasmtime-suite advisories
(CVSS 9.0, 7.5, 6.5, 6.9, 6.1, 5.9, 5.6, 4.1, 3.3, 2.3, 1.8).

The `fke-runtime/src/lib.rs` uses stable component-model APIs that persist across
versions. Expected migration points (verify during implementation):

- `WasiView` trait: `table()` and `ctx()` methods — stable across range
- `wasmtime_wasi_http::bindings::ProxyPre` — verify import path unchanged
- `wasmtime_wasi_http::types::WasiHttpCtx` — verify unchanged
- `Config` methods: `async_support`, `wasm_component_model`, `consume_fuel`,
  `epoch_interruption` — all stable
- `PoolingAllocationConfig` (if used) — check for rename

Workspace changes: `wasmtime = "46"`, `wasmtime-wasi = "46"`,
`wasmtime-wasi-http = "46"` in `[workspace.dependencies]`.

### 2. Upgrade `object_store 0.11 → 0.14`

`object_store 0.14` uses `quick-xml ≥0.41`, resolving both RUSTSEC-2026-0194
and RUSTSEC-2026-0195. Only `fke-store-s3` uses this crate. Check for any
breaking API changes in `fke-store-s3/src/lib.rs`.

### 3. Add `cargo audit` step to CI

Add after the `test` step in `.github/workflows/ci.yml`:

```yaml
- name: Security audit
  run: |
    cargo install cargo-audit --locked
    cargo audit --deny warnings
```

The `--deny warnings` flag treats unmaintained advisories (like `rsa` via
`sqlx-mysql` — no fix available) as errors. Use `.cargo/audit.toml` to
allowlist no-fix advisories with documented justification.

### 4. Write `.cargo/audit.toml`

Allowlist the two no-fix advisories with justification and quarterly expiry:

- `RUSTSEC-2023-0071` (`rsa 0.9.x` via `sqlx-mysql`) — Marvin timing
  side-channel, CVSS 5.9. No fix available upstream. Flint does not use
  RSA key operations directly; this advisory is in `sqlx-mysql`'s dependency
  tree and Flint uses only `sqlx` PostgreSQL features. Allowlist until
  `sqlx` releases a fixed version.
- Any unmaintained crates that have no security impact in Flint's usage paths.
