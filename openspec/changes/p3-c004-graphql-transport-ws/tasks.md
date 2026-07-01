# Tasks — p3-c004-graphql-transport-ws

## Change
GET /graphql WebSocket upgrade via async-graphql-axum GraphQLSubscription

## Status: PENDING (blocked on p3-c007)

---

## Task List

### T1 — Verify async-graphql-axum in fdb-gateway Cargo.toml
- [ ] Confirm `async-graphql-axum = { workspace = true }` is present (added in p3-c007 T7)
- [ ] Add `use async_graphql_axum::GraphQLSubscription;` import to `fdb-gateway/src/main.rs`

### T2 — Add WebSocket extractor to fdb-gateway
- [ ] In `fdb-gateway/src/main.rs`, add `use axum::extract::WebSocketUpgrade;`
- [ ] Verify `axum` workspace dep has WebSocket feature enabled:
  - Check `axum = "0.8.8"` — WebSocket is part of `axum::extract::ws` in Axum 0.8
  - If feature needed: update workspace dep to `axum = { version = "0.8.8", features = ["ws"] }`

### T3 — Implement graphql_ws_handler
- [ ] Write `async fn graphql_ws_handler(...)` per the proposal design
- [ ] Read `compiled.subscription_schema` from `StateManager::current()`
- [ ] Guard on `None` schema → 503 SERVICE_UNAVAILABLE
- [ ] Attempt bearer extraction from `Authorization` header for eager RLS context
- [ ] Pass schema to `GraphQLSubscription::new(schema).serve(socket)`

### T4 — Register GET /graphql route
- [ ] Update the `/graphql` route to combine GET and POST:
  ```rust
  .route("/graphql", get(graphql_ws_handler).post(handle_graphql_query))
  ```

### T5 — Compile and lint gate
- [ ] `cargo check --workspace` — GREEN
- [ ] `cargo clippy --workspace -- -D warnings` — GREEN
- [ ] `cargo test --workspace` — all existing tests pass
- [ ] Mark `p3-c004` as `qa_passed` in `progress.json`
