# p16-c001 Tasks — REST/RPC RLS Enforcement

## Tasks

- [x] Decide the `RestState` seam: verified by reading `fdb_ports::Conn`, `RestExecutor`/`RestQuery`, and `fdb_query::QueryParam` — resolved to a new `SqlExecutor` port (see `proposal.md` §1); rejected a direct `Arc<dyn DatabaseBackend>` (opaque `Conn` can't be downcast outside `fdb-postgres`) and reusing today's `RestExecutor` (too narrow: no embeds/mutations/vector binding)
- [x] Add `QueryParam::Vector(Vec<f32>)` variant to `fdb-query` (non_exhaustive enum; needed for `/rpc` vector-arg binding, see `proposal.md` §2)
- [x] Add `fdb-query` as a dependency of `fdb-ports` (layering-clean: both are pure, dependency-free layer-0/1 crates)
- [x] Add `SqlExecutor` trait to `fdb-ports`: `async fn execute_raw(&self, sql: &str, params: Vec<QueryParam>, rls: &RlsContext) -> Result<Vec<serde_json::Map<String, serde_json::Value>>, BackendError>`
- [x] Implement `SqlExecutor` for `fdb-postgres::PgRest`, factoring the bind/execute/row-projection logic already in `RestExecutor::execute` (`lib.rs:245-266`) into a shared private helper reused by both trait impls
- [x] Handle `QueryParam::Vector` in the bind-mapping helper (`RestBind`/equivalent) using `pgvector::Vector: ToSql` (already available via the `"postgres"` feature)
- [x] Add `Extension<RlsContext>` to `handle_list` in `crates/fdb-reflection/src/compilers/rest/mod.rs` (currently missing)
- [x] Change `RestState` in `fdb-reflection` from `pool: sqlx::PgPool` to `executor: Arc<dyn SqlExecutor>`
- [x] Rewrite `handle_list` to call `state.executor.execute_raw(&sql, inner.binds, &rls).await` instead of `sqlx::query(&sql)...fetch_one(&state.pool)`; adapt row-projection to the new `Vec<Map<String, Value>>` shape
- [x] Rewrite `handle_insert`/`handle_update`/`handle_delete` in `mutations.rs` to use `execute_raw`; convert `json_bind` output to `QueryParam::Json(String)` (serialize + `$n::jsonb` cast) instead of binding `serde_json::Value` natively via sqlx
- [x] Rewrite `handle_rpc` in `rpc.rs` to use `execute_raw`; convert vector args to `QueryParam::Vector`
- [x] Add a non-owner, RLS-subject Postgres role/connection string for the reflection pool (distinct from the migration-owner pool)
- [x] Construct `PgRest`/`Arc<dyn SqlExecutor>` in `fdb-gateway/src/main.rs` against the new non-owner pool; wire into `state_manager.rs::do_compile`, replacing `pool.clone()` at `state_manager.rs:189` for the REST compiler only (GraphQL/vector pools already correct)
- [x] Add migration: `ALTER TABLE ... FORCE ROW LEVEL SECURITY` for all tenant-governed tables; audit `migrations/` for tables with `ENABLE` but not `FORCE`
- [x] Write two-tenant RLS integration test through the real Axum router (GET/POST/PATCH/DELETE/`/rpc`), `DATABASE_URL`-gated
- [x] Assert the test also proves same-tenant access still works (no false-positive isolation from an overly strict policy)
- [x] Correct `mod.rs:62` and `mod.rs:120` doc-comments to describe the fixed behavior
- [x] `cargo check --workspace` clean (compile-economy checkpoint before the full test/clippy pass)
- [x] `cargo clippy --workspace -- -D warnings` clean
- [x] `cargo test --workspace` passes (including the new gated integration test run with `DATABASE_URL` set)
