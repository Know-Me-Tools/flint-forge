# p14-c003 Tasks — A2UI Component Hot-Reload

## Tasks

- [x] Create `migrations/0012_a2ui_change_notify.sql` — `notify_meta_runtime()` function + triggers on `flint_a2ui.components` and `flint_a2ui.applications` (p16-c006 reconcile: corrected filename — the original checkbox said `0010`, which was already taken by `0010_flint_kiln.sql`; content/functionality otherwise matches exactly)
- [x] Add `broadcast_all(event)` method to `AgUiState` in `routes/agui.rs` — iterates all run channels and sends
- [x] Wire `state_manager.subscribe_version()` → `broadcast_all(StateSnapshot)` in `main.rs`
- [x] ~~Add `StateSnapshot { version: u64 }` variant to `AgUiEvent` in `fdb-domain/src/lib.rs`~~ — REUSED existing `StateSnapshot { run_id, state }` variant instead (additive change, no breaking shape edit). Schema version is carried as `state: { "schema_version": version }` with `run_id: "schema"`, matching the existing p7-c007 propagation convention.
- [x] Update `@flint/react` `useFlintRegistry()` — added TODO comment with full integration context (no build attempt per task spec)
- [x] `cargo clippy --workspace -- -D warnings` clean
- [x] `cargo test --workspace` passes (457 tests pass; pre-existing broken `flint-skill` crate excluded — untracked, unrelated in-progress work)
- [x] `docker compose config --quiet` passes (migration ships in repo, applied by `sqlx migrate run --source migrations`)
