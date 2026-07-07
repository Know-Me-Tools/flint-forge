# p14-c003 — A2UI Component Hot-Reload

**Phase:** 14 — v1.1.0  **Priority:** P1  **Depends on:** none

## Problem

When A2UI catalog entries change (INSERT/UPDATE/DELETE on `flint_a2ui.components`),
the `StateManager` does not re-compile and connected AG-UI SSE clients are not
notified. The change requires a service restart.

## Solution

The existing `StateManager` already listens on the `meta_runtime` PostgreSQL
NOTIFY channel for DDL changes and triggers `do_compile()`. This change extends
the notification path to cover A2UI catalog changes.

### Part A: PostgreSQL trigger

Create `migrations/0010_a2ui_change_notify.sql`:

```sql
-- Notify the StateManager when A2UI catalog entries change.
-- The StateManager listens on 'meta_runtime' and triggers do_compile().

CREATE OR REPLACE FUNCTION flint_a2ui.notify_meta_runtime()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM pg_notify('meta_runtime', 'a2ui_change');
    RETURN COALESCE(NEW, OLD);
END;
$$;

CREATE TRIGGER flint_a2ui_components_notify
AFTER INSERT OR UPDATE OR DELETE ON flint_a2ui.components
FOR EACH ROW EXECUTE FUNCTION flint_a2ui.notify_meta_runtime();

CREATE TRIGGER flint_a2ui_applications_notify
AFTER INSERT OR UPDATE OR DELETE ON flint_a2ui.applications
FOR EACH ROW EXECUTE FUNCTION flint_a2ui.notify_meta_runtime();
```

### Part B: AG-UI SSE notification on version change

In `fdb-gateway/src/main.rs` (or a dedicated module), subscribe to
`state_manager.subscribe_version()` and emit an AG-UI event when the version
changes:

```rust
// In main(), after AgUiState is created:
let mut version_rx = state_manager.subscribe_version();
let agui_state_clone = Arc::clone(&agui_state);
tokio::spawn(async move {
    while version_rx.changed().await.is_ok() {
        let version = *version_rx.borrow();
        tracing::info!(version, "schema version changed — notifying AG-UI clients");
        // Broadcast to all connected runs
        agui_state_clone.broadcast_all(AgUiEvent::StateSnapshot { version }).await;
    }
});
```

This requires:
1. A `broadcast_all()` method on `AgUiState` that iterates all run channels
2. A `StateSnapshot` variant on `AgUiEvent` (or reuse an existing event type)

### Part C: SDK auto-refresh

In `@flint/react`, the `useFlintRegistry()` hook should listen for the AG-UI
`StateSnapshot` event and call SWR's `mutate()` to revalidate the component list.
This is a one-line addition to the existing AG-UI event handler in the hook.

### Gate

- Migration applies cleanly
- `StateManager` re-compiles on A2UI catalog change (verified by integration test)
- AG-UI clients receive the version change notification
