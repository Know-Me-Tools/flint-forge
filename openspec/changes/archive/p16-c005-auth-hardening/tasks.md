# p16-c005 Tasks — Auth Hardening

## Tasks

- [x] Replace the `OnceLock` JWKS cache in `forge-identity/src/jwks.rs` with a TTL-aware cache
- [x] Add refetch-on-unknown-`kid` path, rate-limited to avoid refetch storms
- [x] Add configurable TTL (env var, sane default ~10-15 min)
- [x] Add production-mode flag (or reuse existing convention) that fails closed when `FLINT_GATE_AUDIENCE` is unset
- [x] Update `lib.rs:105-109` to enforce mandatory audience in production mode
- [x] Test: JWKS key rotation picked up without process restart
- [x] Test: refetch-on-unknown-kid exercised and rate-limited
- [x] Test: missing audience config fails closed in production mode
- [x] Test: wrong-audience token rejected in production mode
- [x] Document the new env vars in `docs/runbook.md`
- [x] `cargo clippy --workspace -- -D warnings` clean
- [x] `cargo test --workspace` passes
