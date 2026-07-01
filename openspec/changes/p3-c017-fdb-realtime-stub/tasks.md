# Tasks — p3-c017-fdb-realtime-stub

- [ ] 1. Implement reconnect loop with exponential backoff (250ms → 30s cap)
- [ ] 2. Inject service token into tonic channel headers (confirm source: vault vs env)
- [ ] 3. Implement subscriber fan-out (`Vec<Sender<EntityChange>>`)
- [ ] 4. Implement `rls_requery()` against RLS pool with full GUC setup
- [ ] 5. Drop events silently when re-query returns zero rows
- [ ] 6. Gate live `WatchEntityType` on OQ-FRF-1; ship placeholder if unresolved
- [ ] 7. If skipped: record `kbd_stage_handoff_skip` note explaining deferral
- [ ] 8. `cargo check` + clippy + `cargo test -p fdb-realtime`
