# p14-c003 Tasks — A2UI Component Hot-Reload

## Tasks

- [ ] Create `migrations/0010_a2ui_change_notify.sql` — `notify_meta_runtime()` function + triggers on `flint_a2ui.components` and `flint_a2ui.applications`
- [ ] Add `broadcast_all(event)` method to `AgUiState` in `routes/agui.rs` — iterates all run channels and sends
- [ ] Wire `state_manager.subscribe_version()` → `broadcast_all(StateSnapshot)` in `main.rs`
- [ ] Add `StateSnapshot { version: u64 }` variant to `AgUiEvent` in `fdb-domain/src/lib.rs` (if not already present)
- [ ] Update `@flint/react` `useFlintRegistry()` to call SWR `mutate()` on `StateSnapshot` event receipt
- [ ] `cargo clippy --workspace -- -D warnings` clean
- [ ] `cargo test --workspace` passes
- [ ] `docker compose config --quiet` passes (migration embedded)
