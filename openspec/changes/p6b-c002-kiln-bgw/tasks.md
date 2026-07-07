# p6b-c002 Tasks — Kiln BGW

## Tasks

- [ ] Create `crates/fke-server/src/kiln_bgw.rs` — `spawn(pool, runtime, registry)` + `process_batch()` modelled on `fdb-gateway/src/agui_hook_dispatcher.rs`
- [ ] `OutboxRow` struct: `id: i64`, `payload: sqlx::types::Json<serde_json::Value>`, `retry_count: i32`
- [ ] `deliver_kiln_invocation()`: parse `payload["function_name"]` + `payload["function_version"]`, call `registry.resolve()`, load WASM into runtime, call `runtime.handle()`
- [ ] Apply exponential backoff via `apply_retry()` (30 s → 60 s → 120 s → 300 s → fail after 4 retries) — identical to agui_hook_dispatcher
- [ ] Declare `mod kiln_bgw;` in `fke-server/src/main.rs`
- [ ] Spawn the BGW after runtime + registry are constructed: `let _bgw = kiln_bgw::spawn(...)`
- [ ] Unit test: `deliver_kiln_invocation` with a missing function name returns `Err`
- [ ] Unit test: `process_batch` on empty outbox returns `Ok(())` without panicking
- [ ] `cargo clippy -p fke-server -- -D warnings` clean
