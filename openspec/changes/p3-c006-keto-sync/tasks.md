# Tasks — p3-c006-keto-sync

## Change
KetoSyncTask: FRF Iggy keto_changes → flint_meta.keto_tuples

## Status: PENDING (blocked on OQ-8 resolution)

---

## Task List

### T0 — Resolve OQ-8: FRF Iggy keto_changes event type (prerequisite)
- [ ] Check `flint-realtime-fabric` source for Iggy publisher code:
  ```
  find /Users/gqadonis/Projects/prometheus/flint-realtime-fabric -name "*.rs" | xargs grep -l "keto"
  ```
- [ ] Look for: stream name, message schema (`KetoTupleChange` or equivalent), op field values
- [ ] If no Iggy keto topic found: redesign as polling task (see proposal §Design polling-path)
- [ ] Document resolution in `current-waypoint.json` OQ list under OQ-8

### T1 — Add iggy-client to workspace (if OQ-8 = Iggy path)
- [ ] In root `Cargo.toml` `[workspace.dependencies]`, add:
  ```toml
  iggy = "0.6"   # or pinned to version FRF uses
  ```
- [ ] In `crates/fdb-gateway/Cargo.toml`, add:
  ```toml
  iggy = { workspace = true }
  serde_json = { workspace = true }
  ```
- [ ] Run `cargo check --workspace` — GREEN

### T2 — Implement KetoTupleChange types
- [ ] Create `crates/fdb-gateway/src/keto_sync.rs`
- [ ] Add `KetoTupleChange` struct with `serde::Deserialize` (schema confirmed from OQ-8)
- [ ] Add `Op` enum with `Upsert` / `Delete` variants
- [ ] Add `SyncError` error type (using `thiserror`)

### T3 — Implement KetoSyncTask struct and run()
- [ ] Add `KetoSyncTask` struct fields: iggy client, `Arc<dyn DatabaseBackend>`, stream name, consumer group
- [ ] Implement `KetoSyncTask::run()`:
  - Subscribe to FRF Iggy `keto_changes` (or equivalent stream name from OQ-8)
  - Stream messages → deserialize → call `apply_change()`
  - On disconnect: log warn + sleep 5s + retry (infinite loop)
- [ ] Implement `KetoSyncTask::apply_change()`:
  - `Op::Upsert` → INSERT ON CONFLICT DO NOTHING
  - `Op::Delete` → DELETE WHERE exact 4-field match
  - Both run on service-role connection (NOT user RLS context)
- [ ] `#[instrument(skip(self, evt), fields(op = ?evt.op, namespace = %evt.namespace), err)]`
  — do NOT log `subject_id` or `object` (PII risk)

### T4 — Wire background task spawn in main.rs
- [ ] In `crates/fdb-gateway/src/main.rs`, after router construction:
  ```rust
  let keto_sync = KetoSyncTask::new(iggy_client, db.clone(), ...);
  tokio::spawn(keto_sync.run());
  ```
- [ ] Config values (iggy URL, stream name, consumer group, credentials) from env vars
- [ ] Task failure is logged and self-healing — it must NOT crash the gateway process

### T5 — Unit test: KetoSyncTask applies upsert and delete
- [ ] In `crates/fdb-gateway/src/keto_sync.rs` `#[cfg(test)]`:
  - `test_keto_sync_applies_upsert`:
    - Mock `DatabaseBackend` to capture the SQL statement called
    - Build `KeTupleChange { op: Op::Upsert, ... }`
    - Call `task.apply_change(&evt).await`
    - Assert SQL contains `INSERT INTO flint_meta.keto_tuples`
  - `test_keto_sync_applies_delete`:
    - Same pattern; assert SQL contains `DELETE FROM flint_meta.keto_tuples`
- [ ] Run `cargo test -p fdb-gateway` — pass

### T6 — Compile and lint gate
- [ ] `cargo check --workspace` — GREEN
- [ ] `cargo clippy --workspace -- -D warnings` — GREEN
- [ ] `cargo test --workspace` — all tests pass
- [ ] Mark `p3-c006` as `qa_passed` in `progress.json`
