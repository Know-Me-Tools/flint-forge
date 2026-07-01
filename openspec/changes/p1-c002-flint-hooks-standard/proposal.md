# p1-c002 — flint_hooks: standard-tier webhook dispatch

## Why

The existing `flint.dispatch_webhook()` trigger is a stub. The standard tier must fire HTTP webhooks via `pg_net` within the transaction, with HMAC-SHA256 signing (Option-3 from spec §4.2). This is the durable-outbox alternative when sub-second latency is acceptable and retries are not required.

## What

- Implement `flint.dispatch_webhook()` PL/pgSQL trigger body for the `standard` tier:
  - JOIN on `flint.webhooks` WHERE the triggering table/event matches
  - Build payload: `{type, schema, table, record, old_record, timestamp}`
  - Sign payload with HMAC-SHA256 over the webhook's `secret` field
  - Set `X-Forge-Signature: sha256=<hex>` header
  - If `forward_jwt = true`: set `Authorization: Bearer <auth.bearer()>`; else set `Authorization: Bearer <service-jwt>` (see §4 of JWT contract)
  - Merge `custom_headers` JSONB into the outgoing header set
  - Call `net.http_post(target_url, headers, payload, timeout_ms)` for standard tier
  - Route `tier = 'durable'` webhooks to outbox INSERT (implemented in p1-c003)
- Add pgrx integration test: register a webhook, INSERT a row, verify pg_net queue has an entry

## Contract

A `standard` tier webhook registered against `public.messages` fires within the same transaction as an INSERT. `net.http_request_queue` (pg_net's internal table) contains one row after the test INSERT. HMAC-SHA256 signature validates against the webhook secret.

## Out of scope

Retry logic and background delivery (p1-c003). Webhook management REST API (Phase 2).

## Constraints

- `dispatch_webhook()` is `SECURITY DEFINER` — keeps auth context for `auth.bearer()` access
- HMAC-SHA256 must be computed via `pgcrypto.hmac()` — no Rust crypto in this path (pure SQL)
- pg_net must be available in the target Postgres 18 container (verify in p1-c004)
- File size ≤ 500 lines

## Reference

- `crates/ext-flint-hooks/sql/flint_hooks.sql` (existing schema + dispatch stub)
- `docs/contracts/jwt-contract.md` §4 (service-identity token for non-forward-jwt path)
- RFC-FORGE-001 §4.2 (Option-3 forwarding: Authorization + X-Forge-Origin-JWT + X-Forge-Signature)
