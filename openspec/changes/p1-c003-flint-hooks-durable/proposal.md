# p1-c003 — flint_hooks: durable-tier outbox dispatcher

## Why

Some webhooks require guaranteed delivery with retries. The `flint.webhook_outbox` table (already exists) implements the transactional outbox pattern. A background worker polls it with `SELECT ... FOR UPDATE SKIP LOCKED` and delivers with exponential backoff.

## What

- Wire the `durable` tier path in `dispatch_webhook()`: instead of `net.http_post`, INSERT into `flint.webhook_outbox` with `status = 'pending'`
- Implement a pgrx `BackgroundWorker` (`BGW`) as `flint_hooks_bgw`:
  - Poll `flint.webhook_outbox WHERE status = 'pending' AND visible_at <= now() LIMIT 10 FOR UPDATE SKIP LOCKED`
  - For each row: deliver via `net.http_post`; on success: `UPDATE status = 'delivered'`; on failure: increment `retry_count`, set `visible_at = now() + (2^retry_count * interval '1 second')`, set `status = 'pending'` (cap at 10 retries, then `status = 'failed'`)
  - Sleep 1s between poll cycles when queue is empty
- Add `flint.webhook_outbox` GC: delete delivered rows older than 7 days (pg_cron job, registered in p1-c004)
- Add index: `CREATE INDEX ON flint.webhook_outbox (status, visible_at) WHERE status = 'pending'`

## Contract

An INSERT to a `durable`-tier webhook-registered table results in one row in `flint.webhook_outbox` with `status = 'pending'`. The BGW picks it up within 2 seconds in a pgrx test. After mock delivery, `status = 'delivered'`. On mock failure, `retry_count` increments and `visible_at` is bumped by exponential backoff.

## Out of scope

Dead-letter queue, webhook event replay UI (Phase 7+). pg_cron job registration is p1-c004.

## Constraints

- `SELECT ... FOR UPDATE SKIP LOCKED` is the exclusive concurrency primitive — no advisory locks
- pgrx BGW must use `pg_sys::BackgroundWorkerBuilder` — review pgrx 0.18.1 BGW API
- Never log the webhook payload or secret field
- File size ≤ 500 lines — split BGW module into `src/bgw.rs`

## Reference

- `crates/ext-flint-hooks/sql/flint_hooks.sql` (outbox schema)
- pgrx BackgroundWorker documentation
- RFC-FORGE-001 §4.2 (two-tier webhook architecture)
