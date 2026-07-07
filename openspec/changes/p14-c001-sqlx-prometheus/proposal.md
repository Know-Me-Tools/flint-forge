# p14-c001 — sqlx 0.9 Upgrade + Pool Prometheus Metrics

**Phase:** 14 — v1.1.0  **Priority:** P0  **Depends on:** none

## Problem

`sqlx = "0.8"` in workspace deps. `cargo update` for `generic-array` pulls
`sqlx 0.9.0` into pgvector's tree, breaking `Encode`/`Type` trait resolution.
No sqlx pool metrics are emitted — Grafana panel 4 and `HighDbConnections`
alert show "no data".

## Solution

### Part A: Upgrade `sqlx 0.8 → 0.9`

1. Bump `sqlx = "0.9"` in `[workspace.dependencies]` of `Cargo.toml`
2. Run `cargo check --workspace`; fix any API changes (likely none — 0.8→0.9 is minor)
3. Run `cargo update generic-array` — confirm the transitive conflict is resolved
4. Verify pgvector `Encode`/`Type` traits unify: `cargo test -p fdb-reflection`
5. Run `cargo audit` — confirm no new advisories from the sqlx bump

**API surface in use:** `PgPool::connect`, `PgListener::connect_with`, `sqlx::query`,
`sqlx::query_as`, `sqlx::query_scalar`, `sqlx::migrate!`. All stable across 0.8→0.9.

### Part B: Pool metrics emission

Add a background task in `fdb-gateway/src/telemetry.rs` that reads the pool
state every 15 seconds and emits gauges:

```rust
/// Spawn a background loop that emits sqlx pool gauges every 15 seconds.
pub fn spawn_pool_metrics(pool: sqlx::PgPool) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(15));
        loop {
            interval.tick().await;
            let size = pool.size();
            let idle = pool.num_idle();
            metrics::gauge!("sqlx_pool_connections_open").set(size as f64);
            metrics::gauge!("sqlx_pool_connections_idle").set(idle as f64);
        }
    });
}
```

Call from `main()` after pool creation: `telemetry::spawn_pool_metrics(pool.clone());`

The `metrics` crate is already a transitive dependency via `axum-prometheus`.
Add `metrics = "0.24"` to `fdb-gateway/Cargo.toml` explicitly.

### Validation

- `cargo update generic-array` succeeds without breaking pgvector
- `curl http://localhost:8080/metrics | grep sqlx_pool` shows real values
- Grafana panel 4 produces data (when viewed against a live stack)
