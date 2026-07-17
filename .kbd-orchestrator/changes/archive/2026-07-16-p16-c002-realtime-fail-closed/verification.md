# Verification — p16-c002

## Gate
No `Ok(futures::stream::empty())` on any `ChangeStreamSource::watch`, and the
default configuration delivers a real event.

## Evidence to record on completion
- [x] `cargo test -p fdb-realtime` — fabric watch() yields Err(Unavailable)
      (`tests::fabric_watch_returns_unavailable` passes; full suite 25/25 green,
      2026-07-16)
- [x] Subscription with FLINT_CHANGE_SOURCE unset receives an insert event
      (t7 — `listen_change_source_watch_delivers_event` in
      `crates/fdb-realtime/tests/listen_live_pg.rs`, run manually against a
      local Postgres 16 scratch DB per the spec's "verify manually against a
      local Postgres" allowance: `cargo test -p fdb-realtime --test
      listen_live_pg -- --ignored` — 2/2 passed 2026-07-16. Confirms
      `ListenChangeSource::watch()` — the new default — delivers a real
      `ChangeEvent` after an INSERT.)
- [x] FLINT_CHANGE_SOURCE=fabric returns a transport error, not silence
      (`FabricChangeSource::open_frf_stream()` returns
      `Err(StreamError::Unavailable)`; no `Ok(stream::empty())` remains on any
      `ChangeStreamSource::watch` impl. Remaining `stream::empty` hits:
      2× `#[cfg(test)]` fixtures in fdb-app introspection, 1× the GraphQL
      compiler's early-boot no-factory arm — out of criterion-4 scope, flagged
      as follow-up)
- [x] Helm values.yaml:55 path unaffected (pins `listen`, which selects the
      ListenChangeSource branch under the inverted default logic)
- [x] CHANGELOG BREAKING entry present (`## [Unreleased] → ### BREAKING`)

## Additional evidence (2026-07-16)
- `cargo clippy -p fdb-realtime -p fdb-gateway -- -D warnings` — clean
- `cargo fmt -p fdb-realtime -p fdb-gateway --check` — clean
- `.env.example` no longer references "polling"; documents fabric as
  fail-closed opt-in
- `docker-compose.yml` pins `FLINT_CHANGE_SOURCE: listen` on fdb-gateway

## Status
7/7 TASKS COMPLETE. All acceptance criteria satisfied:
1. Default path delivers events — verified live against Postgres 16.
2. `FLINT_CHANGE_SOURCE=fabric` fails closed — unit-verified.
3. Helm unaffected — verified by inspection.
4. No `Ok(stream::empty())` on any `ChangeStreamSource::watch` impl — verified.
5. `.env.example` no longer references "polling" — verified.
