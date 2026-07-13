# p16-c003 Tasks — Kiln Sandbox + Authorization

## Tasks

- [x] Change `check_capabilities` signature to compare requested-vs-granted (`crates/fke-runtime/src/lib.rs:212`)
- [x] Thread the component's declared `capabilities` (from manifest) as the "requested" side of the check
- [x] Build linker capability-scoped instead of unconditional full-WASI (`build_linker`, `fke-runtime/src/lib.rs:404-412`) — DEFERRED: see note below
- [x] Wire `flint:host@0.1.0` capability surface (Db/Llm/Kv/Identity/Secrets) into the linker, gated by the capability check — DEFERRED: no host-side implementation of `flint:host@0.1.0` exists anywhere yet (`flint-skill` is explicitly guest-only SDK scaffolding — "contains no WIT calls of its own"; `wit/flint/host/world.wit` defines the contract but nothing implements the host side). Wiring 5 new host functions (Db/Llm/Kv/Identity/Secrets), each touching a different real backend (Postgres, LLM gateway, ephemeral KV, RlsContext, flint_vault) with Secrets requiring careful default-deny/audit semantics, is substantial new engineering — not a bug fix — and isn't safely completable alongside this change's other P0 items without its own design review and test components to verify against. The actual security bug this change targets (the no-op capability check) is fully fixed independent of this — `build_linker` currently only wires generic WASI/WASI-HTTP (needed by every Kiln function's incoming-handler contract regardless of Flint capabilities), so there is nothing capability-conditional to scope until `flint:host` host functions exist to gate. Flagged as a separate follow-up task.
- [x] Remove `#[allow(dead_code)]` from `KilnHostState.granted` once it's actually read
- [x] Add bearer-verification middleware to `/functions/v1/<name>` (reuse `forge-identity`/`fdb-auth` JWT verify)
- [x] Ensure `caller = None` is unreachable post-auth — missing/invalid bearer is a 401 before the Cedar gate, not a policy-skip path
- [x] Add bearer-verification middleware to `/admin/functions` (not just the `control-plane` compile feature)
- [x] Add admin-scoped claim/role check for `/admin/functions`
- [x] Integration test: ungranted-capability component denied at instantiate
- [x] Integration test: anonymous `/functions/v1/<name>` call is 401
- [x] Integration test: anonymous `/admin/functions` call is 401
- [x] Integration test: well-behaved component using only granted capabilities still runs (no regression) — `granted_capability_passes_check_and_reaches_runtime`
- [x] Coordinate with `flint-skill` WIT bindings if the host-function surface changes — N/A: the host-function surface is unchanged in this PR (deferred, see above)
- [x] `cargo clippy --workspace -- -D warnings` clean
- [x] `cargo test --workspace` passes
