# p16-c003 — Kiln Sandbox + Authorization Enforcement

**Phase:** 16 — Production Remediation
**Priority:** P0 (blocks any production claim)
**Depends on:** p16-c002 (same files: `fke-server/src/main.rs`, `fke-runtime/src/lib.rs`; sequence after to avoid merge conflicts)

## What this change delivers

- Wasmtime linker grants only the WASI/host capabilities a component actually
  declares, not full WASI unconditionally.
- `check_capabilities` compares requested-vs-granted instead of comparing a
  value to itself.
- `flint:host` capability surface (Db/Llm/Kv/Identity/Secrets) gated by Cedar.
- Authentication required on `/functions/v1/<name>` and `/admin/functions`;
  anonymous invocation is denied rather than silently skipping the Cedar gate.

## Problem

`crates/fke-runtime/src/lib.rs::check_capabilities(granted, granted)` (`:212`)
passes the same argument twice — it can never fail, so the capability check is
a no-op. `build_linker` (`:404-412`) unconditionally adds full
`wasmtime_wasi::p2::add_to_linker_async` + `wasmtime_wasi_http` for every
component regardless of declared `capabilities`. `KilnHostState.granted` is
`#[allow(dead_code)]` (`:64-65`) — never read. The `flint:host@0.1.0`
capability surface described in the WIT contract is not wired into the linker
at all.

Separately, `/functions/v1/<name>` requires no auth
(`crates/fke-server/src/main.rs:184-189`): with no bearer, `caller = None`, and
the single Cedar `kiln:invoke` gate at `fke-runtime/src/lib.rs:202-209` is
**skipped entirely** when `caller` is `None` — the exact case for every
anonymous request. `/admin/functions` has no auth middleware at all
(`main.rs:123-128`), gated only by a compile-time feature flag.

## Design

### 1. Real capability enforcement

Change `check_capabilities` to take `(requested: &[Capability], granted: &[Capability])`
and return an error for any requested capability not in `granted`. Call it with
the component's declared `capabilities` (from the manifest) vs. the
caller/tenant's granted set (from Cedar policy or a capability-grant table),
not the same value twice.

### 2. Capability-scoped linker

Build the linker per-invocation (or per capability-set, cached) adding only
the WASI interfaces the component's declared capabilities require. Wire
`flint:host` host functions (Db/Llm/Kv/Identity/Secrets) as real linker
additions gated by the same capability check, replacing the currently-absent
implementation. Coordinate with `flint-skill`'s WIT bindings so the SDK-facing
contract doesn't drift.

### 3. Require authentication on the data and control planes

- `/functions/v1/<name>`: require a valid bearer (reuse the `fdb-auth`/
  `forge-identity` JWT verification already used by Quarry); populate `caller`
  from the verified claims; never allow `caller = None` to skip the Cedar gate
  — treat missing auth as a hard 401, not a policy-skip.
- `/admin/functions`: add the same bearer-verification middleware (not just a
  compile-time feature flag) with an admin-scoped claim/role check.

## Verification (gate)

- Test: a component requesting an ungranted capability (e.g. `Db` without
  grant) is denied at instantiate, not silently allowed.
- Test: anonymous (`no Authorization header`) call to `/functions/v1/<name>`
  is `401`, not executed.
- Test: anonymous call to `/admin/functions` is `401`.
- Test: a component using only its granted capabilities still runs correctly
  (no functional regression for well-behaved components).
