# p7b-c003 — BGW Publisher Identity (Cedar fires on hook invocations)

**Phase:** 7b — Kiln Production Hardening
**Priority:** P0
**Depends on:** p7b-c002 (real Cedar policies — otherwise the gate always allows)
**Blocks:** nothing

## What this change delivers

Changes `kiln_bgw::invoke_function()` to pass a synthetic `RlsContext`
derived from the function's `publisher_did` as the Cedar gate caller.
Currently `caller = None` bypasses Cedar entirely for hook-triggered calls;
after this change the Cedar gate fires for every Kiln invocation.

## Design

### Synthetic `RlsContext` from `publisher_did`

```rust
fn publisher_rls(manifest: &FunctionManifest) -> RlsContext {
    RlsContext {
        role: "kiln_publisher".to_owned(),
        claims_json: format!(r#"{{"sub": "{}"}}"#, manifest.publisher_did),
        raw_bearer: String::new(),
        keto_subject: manifest.publisher_did.clone(),
        vault_key_id: None,
    }
}
```

### Update `invoke_function()`

```rust
// Before (line 134):
None, // BGW = system caller; Cedar gate is skipped

// After:
let publisher = publisher_rls(&manifest);
// ... and pass Some(&publisher) to both handle() calls
```

### Add `forge-identity` direct dep to `fke-server/Cargo.toml`

`forge-identity` is already a transitive dep (via `fdb-auth` → `forge-identity`)
but must be declared directly to use `RlsContext` in `fke-server` code.

## Security note

The `raw_bearer` field is intentionally empty for BGW calls — there is no
JWT to forward. Cedar policy decisions for hook-triggered invocations should
be based on the `publisher_did` (mapped to `keto_subject`), not a user JWT.
