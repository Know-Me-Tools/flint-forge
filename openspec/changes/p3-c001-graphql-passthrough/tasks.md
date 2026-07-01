# Tasks — p3-c001-graphql-passthrough

## Change
POST /graphql → graphql.resolve() under RLS

## Status: PENDING (blocked on p3-c005 + p3-c008)

---

## Task List

### T1 — Extend GatewayState with PgGraphQl
- [ ] In `crates/fdb-gateway/src/main.rs`, add `pg_graphql: Arc<PgGraphQl>` to `GatewayState`
- [ ] In `main()`, construct `PgGraphQl::new(pool.clone())` (need `PgGraphQl` from fdb-postgres)
- [ ] Add `fdb-postgres = { path = "../fdb-postgres" }` to `fdb-gateway/Cargo.toml` if not present (already present — verify)

### T2 — Register POST /graphql route
- [ ] Add `use axum::routing::post;` if not present
- [ ] Register `.route("/graphql", post(handle_graphql_query))` in `fdb-gateway/src/main.rs`
- [ ] Add `use axum::Json;` and `use axum::http::HeaderMap;` imports

### T3 — Implement bearer extraction helper
- [ ] Write `fn extract_bearer(headers: &HeaderMap) -> Option<&str>`:
  - Read `Authorization` header
  - Strip `"Bearer "` prefix
  - Return the remainder as `&str`

### T4 — Implement handle_graphql_query handler
- [ ] Write `async fn handle_graphql_query(...)` per the proposal design
- [ ] Call `fdb_auth::rls_from_bearer(bearer).await` for JWT → RlsContext
- [ ] Call `state.pg_graphql.execute(req, &rls).await`
- [ ] Map errors: auth errors → 401, backend errors → 500
- [ ] Handler MUST NOT log bearer, claims_json, or raw_bearer (use tracing skip)

### T5 — Implement PgGraphQl::execute() body
- [ ] In `crates/fdb-postgres/src/lib.rs`:
  - Add `#[instrument(skip(self, rls), fields(role = %rls.role), err)]` to `execute()`
  - Add `use deadpool_postgres::Object;` or use the existing pool reference
  - Call `PgBackend { pool: self.pool.clone() }.acquire(rls).await` (or refactor to share pool)
  - Downcast `Conn` → `PgConn` via `PgConn::from_conn(&conn)`
  - Execute: `"SELECT graphql.resolve($1::text, $2::jsonb, $3::jsonb)"`
  - Bind: `req.query` (text), `variables` (jsonb from `req.variables.unwrap_or(Null)`), `extensions` (jsonb, Null for now)
  - Map the returned `serde_json::Value` into `Ok(Json(row))`
- [ ] Remove `#[allow(dead_code)]` from `PgGraphQl.pool` once it's used

### T6 — fdb-domain: verify GraphQlRequest has query + variables fields
- [ ] Read `crates/fdb-domain/src/lib.rs` — confirm `GraphQlRequest` has `query: String` and `variables: Option<serde_json::Value>`
- [ ] If missing fields, add them with `#[derive(serde::Deserialize)]`

### T7 — Compile and lint gate
- [ ] `cargo check --workspace` — GREEN
- [ ] `cargo clippy --workspace -- -D warnings` — GREEN
- [ ] `cargo test --workspace` — all existing tests still pass
- [ ] Mark `p3-c001` as `qa_passed` in `progress.json`
