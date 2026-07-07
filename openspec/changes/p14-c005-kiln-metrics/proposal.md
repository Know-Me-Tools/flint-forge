# p14-c005 — Kiln Per-Function Metrics

**Phase:** 14 — v1.1.0  **Priority:** P2  **Depends on:** none

## Problem

`fke-server` has no metrics infrastructure. No `/metrics` endpoint, no
`axum-prometheus` middleware, no Kiln-specific counters. Operators cannot
monitor function invocation rates, fuel consumption, or epoch traps.

## Solution

### Part A: Add `axum-prometheus` to `fke-server`

Same pattern as `fdb-gateway/src/telemetry.rs` (p9-c004):
- Add `axum-prometheus` + `metrics` to `crates/fke-server/Cargo.toml`
- Add `PrometheusMetricLayer` + `/metrics` route to the router
- This gives standard `axum_http_requests_total` + duration histograms

### Part B: Kiln-specific counters

In `fke-server/src/main.rs`, add custom counters inside `invoke_impl()`:

```rust
// At the top of invoke_impl:
metrics::counter!("kiln_invocations_total", "function" => name.clone()).increment(1);

// After invocation completes:
metrics::counter!("kiln_fuel_consumed_total").increment(fuel_used);

// On epoch trap:
metrics::counter!("kiln_epoch_traps_total").increment(1);
```

Fuel consumption requires reading `store.get_fuel()` before and after the
invocation. Epoch traps are detected via the wasmtime error type.

### Gate

- `GET /metrics` on fke-server returns `kiln_invocations_total` and standard axum metrics
- `cargo test -p fke-server` passes
