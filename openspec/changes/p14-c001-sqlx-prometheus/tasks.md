# p14-c001 Tasks — sqlx 0.9 Upgrade + Pool Prometheus Metrics

## Tasks

- [x] Add `metrics = "0.24"` to `crates/fdb-gateway/Cargo.toml`
- [x] Add `spawn_pool_metrics(pool)` function to `crates/fdb-gateway/src/telemetry.rs`
- [x] Call `telemetry::spawn_pool_metrics(pool.clone())` in `main()` after pool creation
- [x] Verify: `cargo check -p fdb-gateway`; `metrics::gauge!` calls compile
- [x] `cargo clippy --workspace -- -D warnings` clean
- [x] `cargo test --workspace` passes

## Still-open debt (p16-c006 reconcile, 2026-07-13) — Part A never shipped

This change had two independent parts; only Part B (pool metrics, above) was ever done. Part A — the actual stated problem this change exists to fix ("cargo update for generic-array pulls sqlx 0.9.0, breaking Encode/Type trait resolution") — was never done:

- [ ] Bump `sqlx = "0.8"` → `sqlx = "0.9"` in `[workspace.dependencies]` of `Cargo.toml` — `Cargo.toml`/`Cargo.lock` still pin `sqlx 0.8.6`; sqlx 0.9.0 is available upstream, so this wasn't blocked by unavailability, it simply never happened.
- [ ] Run `cargo check --workspace`; fix any API changes from 0.8→0.9 — moot until the bump above happens.
- [ ] Run `cargo update generic-array`; confirm pgvector traits still unify — moot until the bump above happens; no transitive conflict currently exists to resolve.
- [ ] Run `cargo test -p fdb-reflection`; confirm vector RPC tests pass — tests do pass today, but against sqlx 0.8, not the 0.9 upgrade this step was meant to validate.
- [ ] `cargo audit` clean — unverified: `cargo-audit` isn't installed in this environment, and the most recent CI `Rust checks` runs on `main` fail at the `fmt` step (pre-existing, unrelated formatting drift) before the audit step would even run. No independent evidence this was ever executed post-bump (and the bump itself didn't happen).

<!-- p16-c006 reconcile (2026-07-13): verified against Cargo.toml/Cargo.lock, crates/fdb-gateway/{Cargo.toml,src/telemetry.rs,src/main.rs}, cargo check/clippy/test --workspace. Confirmed Part A (sqlx 0.9 bump) genuinely never shipped -- this is real, unresolved debt from this change's original stated problem, not rubber-stamped as done. -->
