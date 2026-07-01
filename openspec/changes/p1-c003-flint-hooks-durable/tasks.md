# p1-c003 — Tasks

- [ ] Add pending-outbox index: `CREATE INDEX IF NOT EXISTS webhook_outbox_pending_idx ON flint.webhook_outbox (status, visible_at) WHERE status = 'pending'`
- [ ] Wire durable tier in `dispatch_webhook()`: `ELSIF tier = 'durable' THEN INSERT INTO flint.webhook_outbox (...) VALUES (...)`
- [ ] Add `src/bgw.rs` to `ext-flint-hooks`: implement `BackgroundWorker` struct
- [ ] BGW poll loop: connect to DB, `SELECT ... FOR UPDATE SKIP LOCKED LIMIT 10`
- [ ] BGW delivery: call `net.http_post` per row; UPDATE status on result
- [ ] BGW retry: `UPDATE visible_at = now() + (INTERVAL '1 second' * 2^retry_count), retry_count = retry_count + 1` on failure
- [ ] BGW cap: `UPDATE status = 'failed' WHERE retry_count >= 10`
- [ ] BGW sleep: `pg_sys::WaitLatch(...)` for 1000ms when queue empty
- [ ] Register BGW in `_PG_init()` using pgrx `BackgroundWorkerBuilder`
- [ ] Write pgrx `#[pg_test]` for durable INSERT: register durable webhook, INSERT, assert outbox row exists with status='pending'
- [ ] Write pgrx `#[pg_test]` for retry logic: manually insert outbox row, simulate failure, verify retry_count++ and visible_at bumped
- [ ] Run `cargo pgrx test -p ext-flint-hooks --features pg18` — all tests pass
- [ ] GATE: durable-tier INSERT → outbox row; BGW delivers within 2s; exponential backoff on failure
