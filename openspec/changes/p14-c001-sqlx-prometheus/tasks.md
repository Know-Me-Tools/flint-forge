# p14-c001 Tasks — sqlx 0.9 Upgrade + Pool Prometheus Metrics

## Tasks

- [ ] Bump `sqlx = "0.8"` → `sqlx = "0.9"` in `[workspace.dependencies]` of `Cargo.toml`
- [ ] Run `cargo check --workspace`; fix any API changes from 0.8→0.9
- [ ] Run `cargo update generic-array`; confirm pgvector traits still unify
- [ ] Run `cargo test -p fdb-reflection`; confirm vector RPC tests pass
- [ ] Add `metrics = "0.24"` to `crates/fdb-gateway/Cargo.toml`
- [ ] Add `spawn_pool_metrics(pool)` function to `crates/fdb-gateway/src/telemetry.rs`
- [ ] Call `telemetry::spawn_pool_metrics(pool.clone())` in `main()` after pool creation
- [ ] Verify: `cargo check -p fdb-gateway`; `metrics::gauge!` calls compile
- [ ] `cargo clippy --workspace -- -D warnings` clean
- [ ] `cargo test --workspace` passes
- [ ] `cargo audit` clean
