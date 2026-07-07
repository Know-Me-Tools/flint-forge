# p9-c003 Tasks — Rate Limiting

## Tasks

- [ ] Add `tower-governor = "0.8"` to `[workspace.dependencies]` in `Cargo.toml`
- [ ] Add `tower-governor = { workspace = true }` to `fdb-gateway/Cargo.toml`
- [ ] Read `FLINT_RATE_LIMIT_REST`, `FLINT_RATE_LIMIT_GRAPHQL`, `FLINT_RATE_LIMIT_BURST` env vars in `main.rs`
- [ ] Build `GovernorConfig` for REST and GraphQL rate limits
- [ ] Apply `GovernorLayer` to the app router (after all route definitions, before `.with_state()`)
- [ ] Configure `429` response with `Retry-After` header and JSON body `{"error":"rate limit exceeded","retry_after_secs":N}`
- [ ] Skip rate limiting when limit is `0` (env var unset or set to `0`)
- [ ] Unit test: limit of 1 req/s → second request returns 429
- [ ] Unit test: `FLINT_RATE_LIMIT_REST=0` → rate limiting disabled, requests pass through
- [ ] Integration: add `FLINT_RATE_LIMIT_REST=100` to `.env.example`
- [ ] `cargo clippy -p fdb-gateway -- -D warnings` clean
- [ ] `cargo test -p fdb-gateway` passes
