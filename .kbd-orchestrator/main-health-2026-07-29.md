# main branch health — 2026-07-29

Recorded at the phase boundary, after all phases were reconciled and with
`active_phase: null` (no phase in flight).

Commit: `8f90d5f`

## Gates — all green

| Gate | Result |
|---|---|
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, 0 errors |
| `cargo check --workspace --all-targets` | exit 0, 0 errors |
| `cargo test --workspace` | exit 0 — **570 passed, 0 failed, 6 ignored** |

`scripts/ci-check.sh` runs fmt + clippy + check; all three pass.

## Important qualification on the test run

570 passing is real, but it is **not** full coverage. No Postgres was
running (port 55433 closed — the p17 test container did not survive the
reboot), so every `DATABASE_URL`-gated test early-returned instead of
asserting. 9 of 57 binaries reported 0 assertions, including
`listen_live_pg`. The `#[ignore]`d live-PG tests (6) did not run either.

Consequently these remain **unverified against a live database**:

- `rest_router_extraction` — the missing-`Extension(rls)` fix from
  `71fb5c4` has still never been observed passing against real Postgres.
  It is reasoned from the handler signatures, not measured.
- `gateway_startup_live_pg`, `rest_rls_isolation`,
  `rest_typed_columns_http`, `keto_sync_schema`, the a2ui suite,
  `listen_live_pg`.

To close that gap, start Postgres 18 on 55433 and re-run with
`DATABASE_URL` set, plus `-- --ignored` for the live-PG set.

## Known-open items (not regressions)

- `p12-c001-k6-measure`, `p13-c001-k6-baselines-measure` — `perf/k6/*.js`
  exist; the measurement runs were never performed.
- `p16-c006-selfhost-operator-guide` — no operator guide on disk; p16's
  reflection already discloses this as a PARTIAL gate.
- `cargo pgrx package` fails to link on arm64
  (`_pg_detoast_datum_packed` undefined). No `.so` has been built from
  current source; the `ext-flint-meta` 0.1.1 SQL was validated directly
  against a live Postgres 18 instead.
