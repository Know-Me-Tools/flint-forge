# p16-c001 — Kiln Host Capability Surface (`flint:host@0.1.0`)

**Phase:** 16 — Kiln Capability Wiring  **Priority:** P1  **Depends on:** none (builds directly on landed `fke-runtime` sandbox code)

## Problem

`wit/flint/host/world.wit` defines five interfaces a WASM component can
import — `db`, `llm`, `kv`, `identity`, `secrets` — but nothing in the host
implements any of them:

- [`build_linker`](../../../crates/fke-runtime/src/lib.rs) (fke-runtime/src/lib.rs:404)
  wires only generic WASI + WASI-HTTP into the `Linker<KilnHostState>`. There
  is no `add_to_linker` call for `flint:host`'s `db`/`llm`/`kv`/`identity`/`secrets`
  anywhere in the workspace. A component that imports any of these interfaces
  fails to instantiate today — the capability surface is a paper contract.
- The capability *enforcement* gate is a tautology, not a check:
  [`handle_with_telemetry`](../../../crates/fke-runtime/src/lib.rs) (line 212)
  calls `check_capabilities(granted, granted)` — comparing `granted` against
  itself. `check_capabilities(required, granted)` (line 416) is implemented
  correctly in isolation, but nothing in the codebase ever derives a real
  `required` set from a component's declared imports or a signed manifest.
  There is no `kiln:capability:<name>` Cedar action in `forge-policy::kiln`
  (only `kiln:invoke` and `kiln:register` exist). Grep confirms zero call
  sites for any per-capability Cedar check.
- [`crates/flint-skill`](../../../crates/flint-skill) is confirmed guest-only
  SDK scaffolding (`db.rs`, `llm.rs`, `kv.rs`, `identity.rs`, `secrets.rs`) —
  each file documents itself as "a thin adapter over the WIT-generated
  `bindings::flint::host::*` module" that skill authors implement in their
  own component crate. It contains no host-side implementation and isn't
  meant to.

This is a capability gap, not a currently-exploitable hole (no component
consumes these interfaces yet), but it blocks any real Kiln function from
using governed DB/LLM/secrets access, and the capability-check tautology
would become a real security bug the moment a component does.

## Design

### Part A — Real capability enforcement

1. Add `kiln:capability:<name>` as a Cedar action family in
   `forge-policy::kiln` (mirroring the existing `KILN_INVOKE`/`KILN_REGISTER`
   pattern), one action per `Capability` variant (`Db`, `Llm`, `Kv`,
   `Identity`, `Secrets`, `HttpOutgoing`).
2. Derive the real `required` set from the component's signed/declared
   capability manifest — reuse whatever mechanism `fke-registry` already
   uses to record a published function's requested capabilities (if none
   exists yet, add the minimal field needed: a `Vec<Capability>` stored
   alongside the component at publish time, since Cedar needs *something*
   to check against besides "whatever the caller happened to pass in").
3. Replace `check_capabilities(granted, granted)` with
   `check_capabilities(&required, granted)`, and add a per-capability Cedar
   check via `forge_policy::kiln` before instantiation — default-deny, same
   posture as the existing `kiln:invoke` gate at the top of
   `handle_with_telemetry`.

### Part B — Host trait implementations

Key architectural finding: **`db` and `llm` should share one governed
Postgres connection**, not introduce a second network client into
`fke-runtime`:

- `db.query(sql, params)` → acquires a connection via the existing
  `fdb-ports::DatabaseBackend::acquire(&RlsContext)` port (already
  implemented by `fdb-postgres`, already sets the six `SET LOCAL`
  statements from spec §2.2) and runs the caller-supplied SQL under RLS.
  `fke-runtime` depends on `fdb-ports` (trait) + `fdb-postgres` (adapter) —
  no new backend, no bypass of RLS.
- `llm.complete` / `llm.embed` → **same connection**, calling the existing
  pgrx SQL functions `SELECT llm.complete($1, $2)` / `SELECT llm.embed($1, $2)`
  (`crates/ext-flint-llm/src/sync.rs`), which already route through
  `flint-gate`'s `/v1/llm/complete` and `/v1/llm/embed` (`gate_client.rs`).
  The component never sees a provider key either way — this was already
  true at the SQL layer.
- `kv.get` / `kv.set` → an in-memory `HashMap<String, Vec<u8>>` owned by
  `KilnHostState`, created fresh per `Store` in `handle_with_telemetry`
  (mirrors the existing `granted: Vec<Capability>` field placement), dropped
  when the `Store` drops. No cross-invocation persistence — matches the WIT
  doc comment exactly.
- `identity.origin_jwt` / `identity.claims` → read directly from the
  `caller: Option<&RlsContext>` already passed into `handle_with_telemetry`.
  `origin_jwt` returns `None` unless the publisher's Cedar grant includes
  `kiln:capability:identity` *and* an explicit "may see raw JWT" bit (the
  WIT doc comment calls this default-deny) — `claims` is always available
  once the `Identity` capability itself is granted.
- `secrets.get` / `secret.reveal` → **new work is needed on the Postgres
  side too**: `ext-flint-vault` only has `vault.get_secret` /
  `vault.resolve_api_key`, both `SECURITY DEFINER` and explicitly commented
  as internal-only ("WASM edge components never reach these... use the
  gated `flint:secrets` reveal path" — `ext-flint-vault/src/lib.rs:372-376`).
  That gated path does not exist yet. This change adds a narrowly-scoped
  `vault.reveal_for_kiln(secret_name, publisher_id)` SQL function, callable
  only by the Kiln-service Postgres role, which still writes to
  `vault.access_log` with `action = 'reveal'`. The Rust-side Cedar check
  (`kiln:capability:secrets` + a per-secret resource check) happens
  *before* this function is ever called — Postgres is the audit/decrypt
  boundary, Cedar is the authorization boundary, matching the existing
  layering in spec §2.3.

### Part C — Conditional linker wiring

`build_linker` takes the component's *granted* capability set and only
calls `add_to_linker` for the WIT interfaces present in `granted`. A
component that imports `secrets` without holding the `Secrets` capability
fails at `linker.instantiate_pre` (missing import), not at a runtime call —
fail-closed at load time rather than fail-open at call time.

### Part D — Test component

Extend `examples/hello-component` (or add a sibling example) so at least
one exported handler path calls each of the five interfaces. Without this,
none of the above has an end-to-end proof — today literally zero WASM
components in this repo import `flint:host`.

## Non-goals

- No change to the WIT contract (`wit/flint/host/world.wit`) unless Part D
  surfaces a genuine defect — coordinate with `crates/flint-skill` if so,
  since it's the guest-side mirror of these same interfaces.
- No new Cedar policy *schema* beyond the `kiln:capability:<name>` action
  family — reuses the existing `Pep`/`Decision`/`Request` machinery.
- `HttpOutgoing` capability gating for `wasi:http/outgoing-handler` is
  in scope for Part A/C (it already has a `Capability::HttpOutgoing`
  variant) but not a new interface — WASI-HTTP is already linked.

## Gate

- `cargo test -p fke-runtime` passes, including new tests for: capability
  denial when `required` is not a subset of `granted` (replacing the
  current `check_capabilities(granted, granted)` tautology test), and
  conditional linker wiring (component importing an ungranted interface
  fails to instantiate).
- `cargo test -p forge-policy` passes, including new
  `kiln:capability:<name>` action tests.
- The test component in `examples/` successfully calls all five interfaces
  under `fke-server` when fully granted, and is denied per-interface when
  Cedar/capability set withholds it.
- `cargo clippy --workspace -- -D warnings` clean.
- No `unwrap()`/`expect()` introduced in `fke-runtime`, `fdb-postgres`, or
  `ext-flint-vault` library code (binary/test code exempt per repo rules).
