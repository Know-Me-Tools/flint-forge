# p9-c003 Tasks — Rate Limiting

## Tasks

- [x] Add `tower-governor = "0.8"` to `[workspace.dependencies]` in `Cargo.toml`
- [x] Add `tower-governor = { workspace = true }` to `fdb-gateway/Cargo.toml`
- [ ] Read `FLINT_RATE_LIMIT_REST`, `FLINT_RATE_LIMIT_GRAPHQL`, `FLINT_RATE_LIMIT_BURST` env vars in `main.rs` — p16-c006: `FLINT_RATE_LIMIT_GRAPHQL` is genuinely unread (present in `.env.example` only); `main.rs` only reads `FLINT_RATE_LIMIT_REST`/`FLINT_RATE_LIMIT_BURST`. Flagged as open debt, not fixed here.
- [ ] Build `GovernorConfig` for REST and GraphQL rate limits — p16-c006: only one shared `GovernorConfig` is built and applied to the whole app; there is no separate GraphQL-specific config/limit. Open debt.
- [x] Apply `GovernorLayer` to the app router (after all route definitions, before `.with_state()`)
- [ ] Configure `429` response with `Retry-After` header and JSON body `{"error":"rate limit exceeded","retry_after_secs":N}` — p16-c006: actual body is `{"error":"rate_limit_exceeded","message":<string>}` (different shape, no `retry_after_secs`); no explicit `Retry-After` header set in the error handler. Open debt.
- [x] Skip rate limiting when limit is `0` (env var unset or set to `0`)
- [x] Unit test: limit of 1 req/s → second request returns 429 — `returns_429_when_limit_exceeded`
- [x] Unit test: `FLINT_RATE_LIMIT_REST=0` → rate limiting disabled, requests pass through — `rate_limiting_disabled_when_rps_zero`
- [x] Integration: add `FLINT_RATE_LIMIT_REST=100` to `.env.example`
- [x] `cargo clippy -p fdb-gateway -- -D warnings` clean — confirmed via `cargo clippy --workspace --all-targets -- -D warnings` (superset), clean
- [x] `cargo test -p fdb-gateway` passes — confirmed via `cargo test --workspace` (superset), all green
