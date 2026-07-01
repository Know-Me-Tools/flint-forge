# p2-c005 — StateManager: ArcSwap Hot-Reload via sqlx PgListener

## Change ID
`p2-c005-arcswap-hot-reload`

## Phase
`p2-quarry-reflection-engine`

## Priority
P0 — MVP blocker (hot-reload is a Phase 2 gate criterion)

## Problem Statement

No hot-reload mechanism exists. When the `flint_meta` schema changes (e.g., a
new table is created via the DDL trigger installed by Phase 1), the running
`fdb-gateway` process has no way to pick up the change without a restart.

The Phase 2 gate criterion requires: **zero dropped requests during schema
reload; ArcSwap hot-swap end-to-end within 5 seconds of DDL change**.

## Scope

### In Scope
- `StateManager` struct in `fdb-reflection/src/state_manager.rs`
- `ArcSwap<CompiledState>` as the hot-swap storage primitive
- `StateManager::start_listener()` — background `tokio::task` that listens on
  `meta_runtime` Postgres NOTIFY channel via `sqlx::postgres::PgListener`
- Manual reconnect loop (NOT built into `sqlx::PgListener` — must implement)
- Forced full recompile on reconnect (missed notifications during disconnect)
- Serve old compiled state during recompile (no request downtime)
- Initial compile at startup before accepting requests
- `fdb-gateway` integration: `StateManager::current()` → `Arc<CompiledState>`

### Out of Scope
- Predicate-pushdown optimization for RLS re-check (off by default; operator opt-in)
- Partial recompile (Phase 3 optimization)
- Prometheus metrics for hot-swap latency (Phase 6)

## Design

### StateManager

```rust
// fdb-reflection/src/state_manager.rs
use arc_swap::ArcSwap;
use std::sync::Arc;
use sqlx::PgPool;

use crate::{
    compiled::CompiledState,
    engine::ReflectionEngine,
    compilers::{rest::RestCompiler, openapi::OpenApiCompiler},
    error::ReflectionError,
};

pub struct StateManager {
    compiled: Arc<ArcSwap<CompiledState>>,
    engine: Arc<ReflectionEngine>,
    db_url: String,
}

impl StateManager {
    pub async fn new(engine: ReflectionEngine, db_url: String) -> Result<Self, ReflectionError> {
        let initial = Self::do_compile(&engine).await?;
        Ok(Self {
            compiled: Arc::new(ArcSwap::from_pointee(initial)),
            engine: Arc::new(engine),
            db_url,
        })
    }

    /// Returns the currently active compiled state.
    /// Callers receive an Arc guard that keeps the old state alive until all
    /// in-flight requests using it are done.
    pub fn current(&self) -> Arc<CompiledState> {
        self.compiled.load_full()
    }

    /// Spawn the background listener loop. Returns a JoinHandle so the caller
    /// (fdb-gateway main) can abort it on shutdown.
    pub fn start_listener(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            self.listen_loop().await;
        })
    }

    async fn listen_loop(&self) {
        loop {
            match self.run_listener().await {
                Ok(()) => {
                    // Clean shutdown (should not happen in normal operation)
                    break;
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        "PgListener disconnected; recompiling and reconnecting in 2s"
                    );
                    // Reconnect delay — prevents tight reconnect loops on persistent errors
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                    // Force full recompile on reconnect — notifications may have been missed
                    match Self::do_compile(&self.engine).await {
                        Ok(new_state) => {
                            self.compiled.store(Arc::new(new_state));
                            tracing::info!("schema recompiled after reconnect");
                        }
                        Err(compile_err) => {
                            tracing::error!(error = %compile_err, "recompile failed after reconnect; serving stale state");
                            // Keep serving stale state — do NOT crash the process
                        }
                    }
                }
            }
        }
    }

    async fn run_listener(&self) -> Result<(), sqlx::Error> {
        let mut listener = sqlx::postgres::PgListener::connect(&self.db_url).await?;
        listener.listen("meta_runtime").await?;
        tracing::info!("PgListener connected to meta_runtime channel");

        loop {
            // next() returns Err on connection loss — outer loop catches it
            let notification = listener.recv().await?;
            tracing::debug!(
                channel = notification.channel(),
                payload = notification.payload(),
                "meta_runtime notification received; triggering recompile"
            );

            match Self::do_compile(&self.engine).await {
                Ok(new_state) => {
                    // ArcSwap store is atomic — in-flight requests continue
                    // holding their old Arc guard until they complete
                    self.compiled.store(Arc::new(new_state));
                    tracing::info!("schema hot-swap complete");
                }
                Err(e) => {
                    // Compilation failure — log and continue serving current state
                    // NEVER swap in a failed/empty state
                    tracing::error!(error = %e, "schema recompile failed; serving previous state");
                }
            }
        }
    }

    async fn do_compile(engine: &ReflectionEngine) -> Result<CompiledState, ReflectionError> {
        let model = engine.reflect().await?;
        let router = RestCompiler::compile(&model);
        let openapi_doc = OpenApiCompiler::compile(&model);
        Ok(CompiledState {
            version: model.version,
            database_model: Arc::new(model),
            router: Arc::new(router),
            openapi_doc,
        })
    }
}
```

### Critical Notes on sqlx::PgListener

1. **No built-in auto-reconnect.** `PgListener::recv()` returns `Err` on
   connection loss. The outer `loop` in `listen_loop()` is the reconnect
   mechanism. This is intentional — `sqlx` docs confirm this behavior.

2. **Forced recompile on reconnect.** When the listener reconnects, any
   NOTIFY messages sent during the outage are lost. A forced full recompile
   ensures the state is consistent with the current database regardless of
   what was missed.

3. **`LISTEN` must be re-issued after reconnect.** `PgListener::connect()` +
   `listener.listen()` together establish both the connection and the listen
   registration. The `run_listener()` function handles this atomically each
   reconnect cycle.

4. **Separate DB URL for listener.** The PgListener connection is separate
   from the `deadpool-postgres` query pool. It uses `DATABASE_URL` but
   holds a dedicated long-lived connection. This is by design — Postgres
   LISTEN connections are stateful and cannot be pool-managed.

### ArcSwap Semantics

The `ArcSwap<CompiledState>` ensures that:
- Readers calling `compiled.load_full()` receive an `Arc<CompiledState>` 
  that is guaranteed valid for their entire request lifetime
- When `compiled.store(Arc::new(new_state))` is called:
  - New readers immediately see the new state
  - Old readers continue using their existing `Arc` guard until dropped
  - Old `CompiledState` is deallocated when all old guards drop (typically
    within milliseconds of the last in-flight request completing)
- Zero requests are dropped during the swap

### fdb-gateway Integration

```rust
// crates/fdb-gateway/src/main.rs (additions)
let state_manager = Arc::new(
    StateManager::new(engine, db_url.clone()).await?
);

// Start background listener
let _listener_handle = state_manager.clone().start_listener();

// Dispatch handler reads current compiled router
let router = Router::new()
    .route("/healthz", get(healthz))
    .fallback(move |req: Request| {
        let current = state_manager.current();
        async move { current.router.call(req).await }
    });
```

## Dependencies

`sqlx` must be in `[workspace.dependencies]` with features:
```toml
sqlx = { version = "0.8", features = ["postgres", "runtime-tokio", "uuid", "json"] }
```

This was identified as a gap in the assessment and is added as part of this change.

## Files Affected

| File | Change |
|---|---|
| `crates/fdb-reflection/src/state_manager.rs` | NEW — `StateManager` implementation |
| `crates/fdb-reflection/src/lib.rs` | Export `StateManager` |
| `crates/fdb-gateway/src/main.rs` | Wire `StateManager` into Axum app |
| `Cargo.toml` (root) | Add `sqlx` workspace dep (if not already added by p2-c003) |

## Gate Criteria

Tests in `crates/fdb-reflection/tests/hot_reload.rs`:

- `test_hot_swap_no_dropped_requests` — 100 concurrent requests in flight during
  `StateManager::store()` — all complete successfully, none 500
- `test_listener_reconnects_after_disconnect` — simulate disconnect by killing
  listener conn; assert state is recompiled within 5s
- `test_forced_recompile_on_reconnect` — DDL change while disconnected → state
  reflects change after reconnect
- `test_stale_state_served_on_compile_failure` — inject recompile error → old state
  continues serving (no crash, no swap to empty)
- `test_initial_compile_before_first_request` — `StateManager::new()` blocks until
  first compile completes; `current()` never returns empty state
