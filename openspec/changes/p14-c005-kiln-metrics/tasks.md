# p14-c005 Tasks — Kiln Per-Function Metrics

## Tasks

- [x] Add `axum-prometheus = { workspace = true }` and `metrics = "0.24"` to `crates/fke-server/Cargo.toml`
- [x] Add `telemetry` module or inline in `main.rs`: init `PrometheusMetricLayer::pair()` + `/metrics` route — implemented inline in `main.rs` (the explicitly-offered alternative)
- [x] Add `kiln_invocations_total{function}` counter in `invoke_impl()`
- [x] Add `kiln_fuel_consumed_total` counter — read `store.get_fuel()` before/after — p16-c006 reconcile note: implemented one layer down, in `fke-runtime::EdgeRuntime::handle_with_telemetry` (which `invoke_impl` calls into) rather than literally inside `fke-server::invoke_impl()` — arguably the more correct location since `fke-runtime` owns the wasmtime `Store`
- [x] Add `kiln_epoch_traps_total` counter — detect epoch trap in error handling — same location note as above (`fke-runtime::handle_with_telemetry`)
- [x] `cargo clippy -p fke-server -- -D warnings` clean
- [x] `cargo test --workspace` passes

## Still-open debt (p16-c006 reconcile, 2026-07-13)

- [ ] Update `observability/grafana-dashboard.json` to add a Kiln panel (optional) — genuinely not done (`grep -n "kiln_" observability/grafana-dashboard.json` returns nothing). Explicitly marked optional in its own checkbox text, so low severity, but real.

<!-- p16-c006 reconcile (2026-07-13): verified against crates/fke-server/{Cargo.toml,src/main.rs}, crates/fke-runtime/src/lib.rs, cargo clippy -p fke-server, cargo test --workspace. Functionally complete; the one gap (optional Grafana panel) is tracked above rather than silently checked off. -->
