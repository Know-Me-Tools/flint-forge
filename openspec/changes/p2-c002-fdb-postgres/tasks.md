# Tasks — p2-c002-fdb-postgres

## Change
deadpool-postgres Pool + SET LOCAL RLS Context in `fdb-postgres`

## Status: PENDING

---

## Task List

### T1 — Add workspace dependencies
- [ ] Add `deadpool-postgres = "0.14"` to `[workspace.dependencies]`
- [ ] Add `tokio-postgres = "0.7"` to `[workspace.dependencies]`
- [ ] Verify `cargo check --workspace` passes after dep additions

### T2 — Create `fdb-postgres/src/error.rs`
- [ ] Define `BackendError` with `thiserror` and `#[non_exhaustive]`
- [ ] Variants: `Pool(PoolError)`, `Begin(PgError)`, `SetLocal(PgError)`, `Commit(PgError)`, `PoolBuild`, `InvalidConfig`, `MissingEnv(&'static str)`

### T3 — Create `fdb-postgres/src/conn.rs`
- [ ] Define `Conn<'c>` struct owning `tokio_postgres::Transaction<'c>`
- [ ] Implement `conn.commit() -> Result<(), BackendError>`
- [ ] Implement `conn.tx() -> &Transaction<'_>`
- [ ] Add `Drop` impl note: transaction rolls back on drop (Postgres default — no explicit impl needed; `Transaction::rollback()` is called by `tokio_postgres` on drop)

### T4 — Implement `PgBackend::from_env()`
- [ ] Read `DATABASE_URL` from env; return `MissingEnv` if absent
- [ ] Read `DB_POOL_MAX_SIZE` (default 10) and `DB_POOL_TIMEOUT_SECS` (default 30)
- [ ] Parse `DATABASE_URL` as `tokio_postgres::Config`; return `InvalidConfig` on parse failure
- [ ] Build `deadpool_postgres::Pool` with `RecyclingMethod::Fast`
- [ ] Return `PgBackend { pool }`

### T5 — Implement `PgBackend::acquire()`
- [ ] Checkout from `self.pool.get().await`; return `BackendError::Pool` on failure
- [ ] Begin a `tokio_postgres::Transaction` with `ReadCommitted` isolation
- [ ] Execute `SET LOCAL ROLE $1` with `rls.role` as bound param
- [ ] Execute `SET LOCAL "request.jwt.claims" = $1` with `rls.claims_json` as bound param
- [ ] Build `auth_header` JSON string; execute `SET LOCAL "request.headers" = $1` with it as bound param
- [ ] SECURITY: verify none of the SET LOCAL values appear in tracing spans
- [ ] Return `Conn { tx }` wrapping the open transaction

### T6 — Update `fdb-postgres/Cargo.toml`
- [ ] Add `deadpool-postgres`, `tokio-postgres`, `tokio/full`, `thiserror`, `tracing`

### T7 — Integration tests (require real Postgres)
- [ ] Set up test DB with RLS-enabled test table
- [ ] `test_acquire_sets_rls_role` — `SHOW ROLE` inside transaction returns correct role
- [ ] `test_acquire_sets_jwt_claims` — `SHOW "request.jwt.claims"` returns claims JSON
- [ ] `test_set_local_does_not_escape_tx` — acquire new conn; GUC values not present
- [ ] `test_pool_checkout_on_exhausted_pool` — all connections held → new acquire returns `BackendError::Pool`
- [ ] Mark tests `#[ignore]` if no `DATABASE_URL` in env; document in README

### T8 — Final verification
- [ ] `cargo test -p fdb-postgres` passes (integration tests excluded in CI if no DB)
- [ ] `cargo clippy --workspace -- -D warnings` — no warnings
- [ ] `cargo check --workspace` — clean build
