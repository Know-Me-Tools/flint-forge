# p7b-c001 — Wasmtime Epoch Interruption

**Phase:** 7b — Kiln Production Hardening
**Priority:** P0
**Depends on:** none (all edits in fke-runtime)
**Blocks:** nothing — defence-in-depth alongside existing fuel limit

## What this change delivers

Adds epoch-based timeout to `EdgeRuntime` so a WASM component that loops
forever (or exhausts its fuel via a tight loop) is interrupted within one
epoch tick (~10 ms default). Fuel alone has gaps: a component that performs
many small host calls can evade the fuel limit; epoch interruption catches
everything.

## Design

### Three targeted edits

**1. `EdgeRuntime::new()` — enable epoch interruption in Config:**
```rust
cfg.epoch_interruption(true);
```

**2. `EdgeRuntime::new()` — spawn background ticker:**
```rust
let engine_ref = engine.clone();
let _epoch_ticker = tokio::task::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_millis(10));
    loop {
        interval.tick().await;
        engine_ref.increment_epoch();
    }
});
```
Store the `JoinHandle` in a field to document intent (no abort needed — the
ticker will exit when the engine is dropped and `increment_epoch` panics on
weak ref; or we simply let it run).

**3. `EdgeRuntime::handle()` — set deadline per invocation:**
```rust
store.set_epoch_deadline(1);
```
Set after `store.set_fuel()`. With `epoch_interruption(true)`, this causes
the component to trap the next time the epoch counter is incremented past
the deadline.

### New field on `EdgeRuntime`
```rust
_epoch_ticker: tokio::task::JoinHandle<()>,
```
(Leading underscore suppresses unused-field lint while documenting that the
handle's liveness is required.)

### `KILN_EPOCH_INTERVAL_MS` env var
Make the tick interval configurable for testing: default 10 ms, readable from
`KILN_EPOCH_INTERVAL_MS`. A value of 0 disables the ticker (useful in tests
that want pure fuel control).

## Gate test
Extend the existing `gate_hello_component_returns_http_200` test or add a new
test that:
1. Sets fuel high enough to run
2. Sets `KILN_EPOCH_INTERVAL_MS=1` (1 ms ticks)
3. Verifies the component still returns 200 (not interrupted for fast components)
