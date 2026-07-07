# p6b-c002 — Kiln Background Worker (Hook Outbox Drain)

**Phase:** 6b — Kiln Hardening
**Priority:** P0
**Depends on:** p6b-c001 (Cedar gate must exist before BGW invokes)
**Blocks:** end-to-end `flint_hooks → Kiln → WASM function` path

## What this change delivers

A background polling worker in `fke-server` that drains
`flint.webhook_outbox WHERE target_type = 'kiln'`. Each entry carries a function
name, version, and hook payload; the BGW resolves the function, loads the WASM,
enforces the Cedar gate, invokes `EdgeRuntime::handle()`, and marks the entry
`delivered` or applies exponential backoff on failure.

## Design (mirrors `fdb-gateway/src/agui_hook_dispatcher.rs`)

```rust
// crates/fke-server/src/kiln_bgw.rs

pub fn spawn(pool: Arc<PgPool>, runtime: Arc<EdgeRuntime>, registry: Arc<PgRegistry>) 
    -> tokio::task::JoinHandle<()>

async fn process_batch(pool, runtime, registry) -> Result<()>

// OutboxRow: { id, payload: Json<Value>, agui_run_id (unused), retry_count }
// payload["function_name"], payload["function_version"] → resolve → invoke
```

### Outbox columns used

| Column | Value |
|---|---|
| `target_type` | `'kiln'` |
| `payload` | JSON with `function_name`, `function_version`, `body` |
| `retry_count` | exponential backoff (30 s → 60 s → 120 s → 300 s → fail) |
| `status` | `'pending'` → `'delivered'` or `'retrying'` → `'failed'` |

### `fke-server/src/main.rs` wiring

```rust
let _kiln_bgw = kiln_bgw::spawn(
    Arc::new(pool.clone()),
    Arc::clone(&runtime),
    Arc::clone(&registry),
);
```
