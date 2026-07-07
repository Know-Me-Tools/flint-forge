# p14-c005 Tasks — Kiln Per-Function Metrics

## Tasks

- [ ] Add `axum-prometheus = { workspace = true }` and `metrics = "0.24"` to `crates/fke-server/Cargo.toml`
- [ ] Add `telemetry` module or inline in `main.rs`: init `PrometheusMetricLayer::pair()` + `/metrics` route
- [ ] Add `kiln_invocations_total{function}` counter in `invoke_impl()`
- [ ] Add `kiln_fuel_consumed_total` counter — read `store.get_fuel()` before/after
- [ ] Add `kiln_epoch_traps_total` counter — detect epoch trap in error handling
- [ ] Update `observability/grafana-dashboard.json` to add a Kiln panel (optional)
- [ ] `cargo clippy -p fke-server -- -D warnings` clean
- [ ] `cargo test --workspace` passes
