# p6b-c001 — Cedar Capability Gate for Kiln Runtime

**Phase:** 6b — Kiln Hardening
**Priority:** P0
**Depends on:** forge-policy (already in workspace)
**Blocks:** p6b-c002 (BGW must call gate before invoking)

## What this change delivers

Replaces `fke-runtime`'s pure Rust list-comparison `check_capabilities()` with a
real Cedar policy evaluation. Before `EdgeRuntime::handle()` instantiates a
component, it calls `Pep::check()` with the Kiln action namespace to verify that
the caller's context permits each declared capability.

## Design

### New `forge-policy/src/kiln.rs` (mirrors `a2ui.rs`)

```rust
pub const KILN_RESOURCE: &str  = "kiln:functions";
pub const KILN_INVOKE:   &str  = "kiln:invoke";

pub fn request(action: &str) -> Request {
    Request { action: action.into(), resource: KILN_RESOURCE.into() }
}
```

### Updated `EdgeRuntime`

```rust
pub struct EdgeRuntime {
    engine:        Engine,
    cache:         Mutex<HashMap<ContentId, Arc<CachedComponent>>>,
    fuel_per_call: u64,
    pep:           Option<Arc<dyn Pep>>,   // ← new
}

impl EdgeRuntime {
    pub fn with_pep(mut self, pep: Arc<dyn Pep>) -> Self { ... }
}
```

`handle()` now runs:
1. `pep.check(who, &kiln::request(KILN_INVOKE))` → must be `Decision::Allow`
2. `check_capabilities(manifest.capabilities, granted)` → verify intersection
3. Proceed to instantiation only on both passes

### `fke-server/src/main.rs` wiring

Inject `CedarPolicyEngine::new(policy_source)` into `EdgeRuntime::with_pep()`.

## Security

- A WASM function cannot run if Cedar denies `kiln:invoke`, even if it is in the
  registry and all capabilities appear granted.
- Capability intersection gate remains; Cedar is an additional enforcement layer.
