# p1-c011 — flint_meta gate test: sqlx PgListener receives NOTIFY on DDL

## Why

This is the P1 phase gate: proof that the full NOTIFY pipeline works end-to-end. A test that creates a table, listens on `meta_runtime`, and receives the notification within 5 seconds confirms that `ext-flint-meta` (p1-c007 through p1-c010) is correctly wired and the `flint-reflection` StateManager reconnect loop in Phase 2 will have a real event stream to consume.

## What

An integration test in `crates/fdb-app/tests/` (or a new `crates/fdb-reflection/tests/` directory created ahead of Phase 2) using sqlx and tokio:

```rust
#[tokio::test]
async fn meta_listener_receives_notify_on_create_table() {
    let pool = test_pool_with_ext_flint_meta().await;

    let initial_version: i64 = sqlx::query_scalar("SELECT flint_meta.version()")
        .fetch_one(&pool).await.unwrap();

    // Start listener BEFORE the DDL
    let mut listener = sqlx::postgres::PgListener::connect_with(&pool).await.unwrap();
    listener.listen("meta_runtime").await.unwrap();

    // DDL in a separate connection
    sqlx::query("CREATE TABLE IF NOT EXISTS public.meta_gate_test_001 (id uuid PRIMARY KEY DEFAULT gen_random_uuid())")
        .execute(&pool).await.unwrap();

    // Expect notification within 5s
    let notification = tokio::time::timeout(
        Duration::from_secs(5),
        listener.recv(),
    ).await
        .expect("notification not received within 5s")
        .expect("listener error");

    let payload: serde_json::Value = serde_json::from_str(notification.payload()).unwrap();
    assert!(payload["version"].as_i64().unwrap() > initial_version);
    assert_eq!(payload["ddl_tag"].as_str().unwrap(), "CREATE TABLE");

    let new_version: i64 = sqlx::query_scalar("SELECT flint_meta.version()")
        .fetch_one(&pool).await.unwrap();
    assert!(new_version > initial_version);

    // Cleanup
    sqlx::query("DROP TABLE IF EXISTS public.meta_gate_test_001")
        .execute(&pool).await.unwrap();
}

#[tokio::test]
async fn meta_listener_reconnect_forces_recompile() {
    // Simulates the StateManager reconnect path:
    // 1. Connect PgListener, listen on meta_runtime
    // 2. Artificially drop the connection (pg_terminate_backend on listener PID)
    // 3. Reconnect and re-listen
    // 4. Create a table — verify notification received on new connection
    // This test verifies that reconnect + re-LISTEN works correctly.
}
```

## Contract

`cargo test -p fdb-app --test meta_listener` (or equivalent test path) passes within CI timeout. Both tests pass: DDL notification received within 5s, reconnect path works.

## Out of scope

The full `fdb-reflection` StateManager (Phase 2). The reconnect backoff logic (implemented in Phase 2 as part of the Rust `StateManager`). This change only validates the NOTIFY → LISTEN round trip at the database level.

## Constraints

- Test requires `ext-flint-meta` installed on a Postgres 18 instance with a real event trigger firing
- Test must clean up its own DDL (`DROP TABLE IF EXISTS`) in teardown
- Never assert notification payload content that could contain schema/tenant data in production — test uses a dedicated test table name
- Add the test crate to workspace `members` if creating a new crate — it is NOT a pgrx crate, so it can be in the workspace

## Reference

- sqlx `PgListener` API: `connect_with`, `listen`, `recv`, `into_stream`
- `docs/FLINT-PHASE-PLAN-REVISED.md` §Phase 1 p1-c011 (gate test spec with reconnect loop)
- `docs/FLINT-META-EXTENSION-PLAN.md` §4.5 (StateManager reconnect loop code)
