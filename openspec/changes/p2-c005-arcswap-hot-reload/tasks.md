# Tasks — p2-c005-arcswap-hot-reload

## Change
StateManager: ArcSwap<CompiledState> hot-reload via sqlx PgListener

## Status: PENDING
## Depends On: p2-c003 (CompiledState, ReflectionEngine), p2-c004 (RestCompiler)

---

## Task List

### T1 — Verify sqlx workspace dep includes PgListener features
- [ ] Confirm `sqlx` in `[workspace.dependencies]` has `features = ["postgres", "runtime-tokio", ...]`
- [ ] `cargo check --workspace` passes with `use sqlx::postgres::PgListener`

### T2 — Create `src/state_manager.rs`
- [ ] Define `StateManager { compiled: Arc<ArcSwap<CompiledState>>, engine: Arc<ReflectionEngine>, db_url: String }`
- [ ] Implement `StateManager::new(engine, db_url) -> Result<Self, ReflectionError>`
  - Call `do_compile()` for initial state
  - Wrap in `Arc::new(ArcSwap::from_pointee(initial))`
  - Return `Self` — process must NOT start serving before this returns

### T3 — Implement `StateManager::current()`
- [ ] `fn current(&self) -> Arc<CompiledState>`
- [ ] Returns `self.compiled.load_full()` — returns `Arc` that keeps old state alive for request duration

### T4 — Implement `StateManager::do_compile()`
- [ ] `async fn do_compile(engine: &ReflectionEngine) -> Result<CompiledState, ReflectionError>`
- [ ] Call `engine.reflect()` → `DatabaseModel`
- [ ] Call `RestCompiler::compile(&model)` → `axum::Router`
- [ ] Call `OpenApiCompiler::compile(&model)` → `serde_json::Value`
- [ ] Return `CompiledState { version, database_model: Arc::new(model), router: Arc::new(router), openapi_doc }`

### T5 — Implement `StateManager::start_listener()`
- [ ] `fn start_listener(self: Arc<Self>) -> tokio::task::JoinHandle<()>`
- [ ] Spawn `tokio::task` calling `self.listen_loop()`
- [ ] Return handle so `fdb-gateway` can abort on shutdown

### T6 — Implement `listen_loop()` — outer reconnect loop
- [ ] Outer `loop { match self.run_listener().await { ... } }`
- [ ] On `Ok(())`: `break` (clean exit — should not happen in normal operation)
- [ ] On `Err(e)`: `tracing::error!` (error code only, no JWT values), sleep 2s, force recompile
- [ ] On force-recompile failure: `tracing::error!` and continue loop (serve stale state, do NOT crash)

### T7 — Implement `run_listener()` — inner listen loop
- [ ] `async fn run_listener(&self) -> Result<(), sqlx::Error>`
- [ ] `sqlx::postgres::PgListener::connect(&self.db_url).await?`
- [ ] `listener.listen("meta_runtime").await?`
- [ ] `tracing::info!` "PgListener connected to meta_runtime channel"
- [ ] Inner `loop { let notif = listener.recv().await?; ... trigger recompile ... }`
- [ ] On `recv()` error: propagate to outer loop (reconnect handles it)
- [ ] On recompile success: `self.compiled.store(Arc::new(new_state))` + `tracing::info!`
- [ ] On recompile failure: `tracing::error!` + continue (do NOT swap in empty state)

### T8 — Wire StateManager into `fdb-gateway/src/main.rs`
- [ ] Create `ReflectionEngine::new(pool)` from sqlx PgPool (service_role URL)
- [ ] `StateManager::new(engine, db_url).await?` — blocks until initial compile
- [ ] `state_manager.clone().start_listener()` — background task
- [ ] Add `GET /openapi.json` route reading from `state_manager.current().openapi_doc`
- [ ] Add fallback route: dispatch to `state_manager.current().router`
- [ ] Update `fdb-gateway/Cargo.toml` to add `fdb-reflection` path dep

### T9 — Export `StateManager` from `fdb-reflection/src/lib.rs`
- [ ] `pub use state_manager::StateManager;`

### T10 — Integration tests `tests/hot_reload.rs`
- [ ] `test_hot_swap_no_dropped_requests` — 100 concurrent requests during `store()` — all complete, none 500
- [ ] `test_listener_reconnects_after_disconnect` — kill listener connection; state recompiled within 5s
- [ ] `test_forced_recompile_on_reconnect` — DDL change while disconnected → new table visible after reconnect
- [ ] `test_stale_state_served_on_compile_failure` — inject compile error; old state continues serving
- [ ] `test_initial_compile_blocks_before_serving` — `StateManager::new()` completes before first request

### T11 — Final verification
- [ ] `cargo test -p fdb-reflection -- hot_reload` passes
- [ ] `cargo test -p fdb-gateway` passes (basic healthz + openapi route)
- [ ] `cargo clippy --workspace -- -D warnings` — no warnings
- [ ] `cargo check --workspace` — clean build
