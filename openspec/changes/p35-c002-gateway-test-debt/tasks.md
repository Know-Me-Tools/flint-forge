# Tasks — p35-c002-gateway-test-debt

- [x] Extract pure `resolve_interval(Option<&str>) -> Duration` from `keto_sync_config_from_env`.
- [x] Rewrite the 3 env-mutating keto_sync tests to exercise the pure fn (no set_var → no race).
- [x] (a2ui_seed_test uninlined_format_args already cleared under c001 gate-unblock.)
- [x] Verify `cargo test -p fdb-gateway` green + deterministic under parallel runs.
