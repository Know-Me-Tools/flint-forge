# p16-c004 Tasks — Realtime Delivery By Default

## Tasks

- [x] Check FRF team status on `WatchEntityType` RPC (OQ-FRF-1) before choosing (a) vs (b) — no channel to the team exists in this environment; decided from repo evidence (`OQ-FRF-1` comments confirm the RPC hasn't landed) — see `proposal.md`
- [x] Flip default in `crates/fdb-gateway/src/main.rs:601` from `fabric` to `listen`
- [x] Update the doc comment at `main.rs:598-600` to match the chosen default/behavior
- [x] Set `FLINT_CHANGE_SOURCE=listen` (or chosen default) in `docker-compose.yml`
- [x] Set `FLINT_CHANGE_SOURCE=listen` in `docker-compose.prod.yml`
- [x] Set `FLINT_CHANGE_SOURCE=listen` in `docker-compose.staging.yml`
- [x] Confirm `deploy/helm/flint-forge/values.yaml:55` still matches after the default change (avoid now-redundant override, or leave as explicit documentation) — confirmed already `"listen"`, kept as explicit documentation
- [x] Integration test: subscription on default config receives a real `ChangeEvent` after a row mutation — split into two precise tests: `fdb-gateway::realtime_source` unit tests prove the default (no env var) resolves to `Listen`, and the existing DATABASE_URL-gated `listen_change_source_watch_delivers_event` (`fdb-realtime/tests/listen_live_pg.rs`) proves `ListenChangeSource` (what's selected by default) delivers a real event after a row mutation
- [x] Grep workspace for any other silent-empty-stream fallback and fix or document each — only `FabricChangeSource::watch()` (already fixed: no longer default, already logs loudly) has this pattern; `compilers/graphql.rs`'s empty stream is a documented fail-closed security behavior (missing RLS context), not the same bug; `introspection.rs`'s two sites are test-only fixtures; confirmed no other `ChangeStreamSource` implementor exists
- [x] `cargo clippy --workspace -- -D warnings` clean
- [x] `cargo test --workspace` passes
