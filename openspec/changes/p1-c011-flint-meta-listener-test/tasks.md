# p1-c011 — Tasks

## Pre-implementation
- [ ] Determine where to place the integration test: `crates/fdb-app/tests/` OR create `crates/fdb-reflection/` stub (prefer fdb-app to avoid creating Phase 2 crate prematurely)
- [ ] Confirm `DATABASE_URL` env var setup for integration tests in this workspace (check existing test patterns in `crates/fdb-*`)
- [ ] Confirm `ext-flint-meta` is installed on the test Postgres 18 instance (Docker compose or pg_cron container)

## Test implementation
- [ ] Write `crates/fdb-app/tests/meta_listener.rs` (or equivalent path):
  - [ ] `test_pool_with_ext_flint_meta()` helper: connect to test DB, run `CREATE EXTENSION IF NOT EXISTS ext_flint_meta`
  - [ ] `meta_listener_receives_notify_on_create_table` test (full implementation per proposal)
  - [ ] `meta_listener_reconnect_forces_recompile` test:
    - [ ] Connect listener, get PID
    - [ ] DROP and re-open pool (simulate disconnect)
    - [ ] Re-listen on meta_runtime
    - [ ] CREATE TABLE → assert notification received on reconnected listener
- [ ] Add `sqlx = { version = "0.8", features = ["postgres", "runtime-tokio-native-tls"] }` to `fdb-app/Cargo.toml` if not already present (check workspace.dependencies first — BLOCK on adding without checking)
- [ ] Add `tokio = { version = "1", features = ["full"] }` as dev-dependency if needed (likely already in workspace)

## Documentation
- [ ] Add comment in test explaining why we re-LISTEN after reconnect (PgListener does not auto-resubscribe)
- [ ] Add `// SECURITY: test table name is deterministic and non-sensitive` note

## Verification
- [ ] Set `DATABASE_URL=postgres://...` pointing at test PG18 with ext-flint-meta installed
- [ ] `cargo test -p fdb-app --test meta_listener -- --nocapture` — both tests pass
- [ ] Notification arrives within 5s (not timeout)
- [ ] Version counter increments
- [ ] GATE: both listener tests pass; DDL notification pipeline confirmed end-to-end

## Notes

- This is the **phase gate test** — it must pass before Phase 2 begins
- The reconnect test verifies the StateManager design assumption: on connection loss, must re-LISTEN explicitly (sqlx PgListener does not auto-resubscribe)
- If running in CI without a real Postgres, skip with `#[ignore]` and note that it requires a running PG18 + ext-flint-meta
