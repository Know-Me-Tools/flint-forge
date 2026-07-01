# p1-c002 — Tasks

- [ ] Read `crates/ext-flint-hooks/Cargo.toml` — confirm pgrx version; upgrade to 0.18.1 if on older version (pg18 target)
- [ ] Read `crates/ext-flint-hooks/sql/flint_hooks.sql` in full — understand current schema
- [ ] Confirm `net.http_post` / `net.http_request` function signature in target PG18 container
- [ ] Implement `dispatch_webhook()` trigger body — step 1: query matching webhooks
  ```sql
  SELECT w.* FROM flint.webhooks w
  WHERE w.schema_name = TG_TABLE_SCHEMA
    AND w.table_name  = TG_TABLE_NAME
    AND TG_OP = ANY(w.events)
    AND w.active = true
  ```
- [ ] Implement payload build: `jsonb_build_object('type', TG_OP, 'schema', TG_TABLE_SCHEMA, 'table', TG_TABLE_NAME, 'record', to_jsonb(NEW), 'old_record', to_jsonb(OLD), 'timestamp', now())`
- [ ] Implement HMAC-SHA256 signing via `pgcrypto.hmac(payload::text, secret, 'sha256')`; format as `sha256=<encode(digest, 'hex')>`
- [ ] Implement header assembly: `X-Forge-Signature`, `Authorization`, `X-Forge-Origin-JWT` (when forward_jwt=true), merge `custom_headers`
- [ ] Implement tier routing: `IF tier = 'standard' THEN net.http_post(...)` (durable INSERT placeholder for p1-c003)
- [ ] Add trigger binding SQL: `CREATE OR REPLACE TRIGGER flint_dispatch` template that users attach to their tables
- [ ] Write pgrx `#[pg_test]` for standard dispatch: register webhook, INSERT row, verify `net.http_request_queue` has entry (or use pg_net mock)
- [ ] Run `cargo pgrx test -p ext-flint-hooks --features pg18` — all tests pass
- [ ] GATE: standard-tier webhook fires pg_net call; HMAC signature header present
