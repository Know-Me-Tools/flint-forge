# p16-c001 — REST/RPC RLS Enforcement (Tenant Isolation)

**Phase:** 16 — Production Remediation
**Priority:** P0 (blocks any production claim)
**Depends on:** none

## What this change delivers

- Every `fdb-reflection` REST CRUD handler (`GET/POST/PATCH/DELETE`) and
  `POST /rpc/<schema>/<fn>` runs inside a per-request transaction with the
  three `SET LOCAL` GUCs applied (`ROLE authenticated`,
  `request.jwt.claims`, `request.headers`) — the same contract §2.2 already
  requires and that `/graphql` already honors.
- A distinct, non-owner, RLS-subject connection pool for the reflection
  router, separate from the migration-owner pool.
- `FORCE ROW LEVEL SECURITY` on tenant tables via a new migration.
- A `DATABASE_URL`-gated integration test that seeds two tenants and proves
  tenant A cannot read or mutate tenant B's rows through the real Axum HTTP
  router, for every verb.
- Corrected doc-comments (the current `mod.rs:62`/`:120` claims are false —
  fixed in p16-c006, referencing this change).

## Problem

`crates/fdb-reflection/src/compilers/rest/{mod,mutations,rpc}.rs` execute
`sqlx::query(...).fetch_*(&state.pool)` directly. `state.pool` traces back to
`PgPool::connect(DATABASE_URL)` in `crates/fdb-gateway/src/main.rs:88` — the
same pool that runs migrations and is explicitly documented elsewhere in
`main.rs` as "MUST NOT be the per-user RLS pool." The `require_rls` middleware
(`crates/fdb-gateway/src/rls_layer.rs:31`) verifies the bearer and inserts
`RlsContext` into request extensions, but nothing downstream ever issues
`SET LOCAL ROLE` / `set_config('request.jwt.claims', ...)` on the connection
that runs the query. If `DATABASE_URL` owns the tables (the normal case, since
it runs migrations), Postgres RLS does not apply to the owner and **any
authenticated user can read or write any tenant's rows** via REST.

Only `/graphql` and `/rpc/vector` are safe today, because they route through
`PgGraphQl`/`PgVectorRpc` → `PgBackend::acquire()`
(`crates/fdb-postgres/src/lib.rs:62-107`), which correctly issues the GUCs
inside a transaction per request.

## Design

### 1. The RestState seam — resolved by investigation, not sketch

Two options were sketched before implementation began; both turned out to be
wrong once the actual types were read, and a third emerged as the only one
that is both layering-compliant and feature-complete:

- **Rejected — `Arc<dyn DatabaseBackend>` directly in `RestState`.**
  `fdb_ports::Conn` is `Box<dyn std::any::Any + Send>` — a deliberately opaque
  handle. Only `fdb-postgres::PgConn::from_conn` can downcast it back to a
  runnable `tokio_postgres::Object` (`crates/fdb-postgres/src/conn.rs:20-23`).
  `fdb-reflection` does not and must not depend on `fdb-postgres` (that's the
  adapter crate; `fdb-reflection` sits at the same layer as `fdb-app` per
  `CLAUDE.md`'s hexagonal rule: "Domain and app crates must never import
  adapter crates"). So `fdb-reflection` can `acquire()` a connection but could
  never actually run a query on it.
- **Rejected — reuse the existing `RestExecutor`/`RestQuery` port.**
  `fdb-postgres::PgRest` already implements `RestExecutor` and already calls
  `backend.acquire(rls)` correctly (`crates/fdb-postgres/src/lib.rs:227-266`)
  — this is the exact working pattern to copy. But `RestQuery`
  (`crates/fdb-domain/src/lib.rs:42-50`) is a flat `(column, op, value)` AND-only
  filter list with no `select` embed grammar, no mutation support, and no
  vector-arg binding. `fdb-reflection`'s compiler needs all three (resource
  embedding via `fdb_query::embed`, `/rpc` with `pgvector::Vector` binding).
  Swapping onto today's `RestExecutor` would regress functionality.
- **Chosen — a new `SqlExecutor` port, one layer lower than `RestExecutor`.**
  Add `fdb_ports::SqlExecutor`, sitting *below* `RestQuery`/`RestExecutor`
  semantically: it takes already-rendered SQL + bound params (produced by
  `fdb_query`, exactly as the reflection compiler already builds them) and
  executes them inside `PgBackend::acquire(rls)`'s transaction, returning raw
  rows. `fdb-postgres::PgRest` implements it by factoring out the
  bind/execute/project logic it already has in `RestExecutor::execute`
  (`lib.rs:245-266`) into a shared helper. `fdb-ports` gaining a dependency on
  `fdb-query` is layering-clean: `fdb-query` is explicitly a pure, dependency-free
  layer-0/1 crate ("no database dependency and no async", per its own module
  doc) — the same tier as `fdb-domain`, not an adapter.

  `RestState` gets `executor: Arc<dyn SqlExecutor>` instead of `pool: sqlx::PgPool`.
  Every handler keeps its existing `fdb_query`-based SQL-building logic
  unchanged and calls `state.executor.execute_raw(&sql, binds, &rls).await?`
  instead of `sqlx::query(&sql)...fetch_*(&state.pool)`.

### 2. Parameter-type gap: `pgvector` binding

`rpc.rs`'s vector-argument binding (`q.bind(vec)` where `vec: pgvector::Vector`,
`crates/fdb-reflection/src/compilers/rest/rpc.rs:78`) has no equivalent in
`fdb_query::QueryParam`. Since `QueryParam` is `#[non_exhaustive]` and
explicitly documents itself as "deliberately backend-agnostic" — mapped onto a
concrete bind by the executor adapter — add a `Vector(Vec<f32>)` variant.
`pgvector = { version = "0.4", features = ["postgres", "sqlx"] }` already
enables the `tokio-postgres`-compatible `"postgres"` feature
(`Cargo.toml:46`), so `pgvector::Vector: ToSql` is available to the new
`PgRest`-side bind mapping with no new dependency.

Mutation JSON binding (`mod.rs::json_bind`, currently a passthrough
`serde_json::Value` bound directly via sqlx's native jsonb support) is
re-expressed as `QueryParam::Json(String)` (serialize then `$n::jsonb` cast in
SQL) — the same convention already used by the containment/JSON-path
operators, rather than introducing a second native-JSON bind path.

### 3. Row-decode gap discovered during implementation: JSON/BIGINT, not text

Every reflection-compiler handler wraps its output as a single `json`/`jsonb`
column (`row_to_json(<table>)` for insert/update/rpc, `json_agg(t)` for list),
with `handle_list` adding a `bigint` `total` sidecar. `postgres-types`'
`String: FromSql::accepts()` only matches `TEXT`/`VARCHAR`/`BPCHAR`/`NAME`/
`UNKNOWN` — **not** `JSON`/`JSONB`/`INT8`/etc. — so a naive `Option<String>`
row-decode (the pattern already used by `PgVectorRpc::execute_similarity` and
the pre-existing `RestExecutor::execute`) would silently return `null` for
every column that actually carries the response, breaking every endpoint's
output while still returning `200`. `execute_raw`'s row-projection
type-dispatches on `col.type_()` (JSON/JSONB → `serde_json::Value` via the
workspace's already-enabled `tokio-postgres "with-serde_json-1"` feature;
BOOL/INT2/INT4/INT8/FLOAT4/FLOAT8 → native decode; else falls back to text)
rather than reusing the existing text-only pattern. This is a genuine, latent
bug in the pre-existing `PgVectorRpc`/`RestExecutor` row-projection code
(any non-text column returns `null` there too) — out of scope to fix on those
paths in this change, but flagged for `kbd-reflect` as follow-up debt since it
likely also affects `PgGraphQl::execute`'s `let raw: String = row.get(0)` read
of `graphql.resolve()`'s `jsonb` return value.

`handle_list` currently doesn't even receive `Extension<RlsContext>`
(`mod.rs:128-133`) — add it, matching the mutation handlers.

### 2. Pool design — resolved by verification, not assumption

Initially assumed a *second Postgres login role/credential* was required for
correctness. Verified against the actual schema and found this is not the
case: `crates/ext-flint-auth/sql/flint_auth.sql:8-15` already provisions
`authenticated`/`anon`/`service_role` as plain `CREATE ROLE ... NOLOGIN` —
no `BYPASSRLS`, no table ownership, no superuser. Postgres's `SET LOCAL ROLE`
(issued inside `PgBackend::acquire`) genuinely de-escalates the session's
effective privilege to that role for the transaction, regardless of what the
physical connection originally authenticated as — this is the same mechanism
PostgREST/Supabase rely on, and it is real here, not aspirational. **The
`acquire()` wiring itself is therefore the actual fix**; a distinct DB-level
login credential is a defense-in-depth improvement (blast-radius reduction if
`SET ROLE` ever silently no-ops), not a correctness requirement, and would be
a separate, larger infra change (new role + deployment secrets) out of this
surgical fix's scope.

What still matters, and is what "non-owner pool" concretely means here: the
REST/`rpc` executor must be a `PgBackend`/`PgRest`-wrapped pool that always
goes through `acquire()` — never the raw, ambient `sqlx::PgPool` used for
migrations/A2UI-seed/embedder writes in `fdb-gateway/src/main.rs`, which never
sets RLS GUCs at all. `crates/fdb-gateway/src/main.rs` already builds two such
`PgBackend`-wrapped pools this way for `graphql_executor`/`vector_rpc`
(`main.rs:125-147`, same `database_url`, separate `deadpool_postgres::Pool`
objects) — the REST executor follows the identical, already-correct
convention: a **third** such pool, not a new database role.

### 3. `FORCE ROW LEVEL SECURITY`

Add a migration that runs `ALTER TABLE ... FORCE ROW LEVEL SECURITY` for every
RLS-governed tenant table, so even a future owner-role misconfiguration
degrades safely instead of silently bypassing policy. Confirm which tables in
`migrations/` currently have `ENABLE ROW LEVEL SECURITY` without `FORCE`.

### 4. Fix or remove the misleading docs (this change or p16-c006)

`mod.rs:62` ("CRUD handlers remain `todo!()` stubs") and `mod.rs:120` ("RLS is
enforced by the connection's GUC context") are both false today and must not
ship still-false after this change lands.

## Verification (gate)

A new `DATABASE_URL`-gated integration test (pattern after
`crates/fdb-postgres/tests/pgrest_live_pg.rs`, but through the **real HTTP
router**, not the bare executor):

1. Seed two tenants (`tenant_a`, `tenant_b`) with an RLS policy scoping rows by
   `tenant_id`.
2. Mint a JWT for a `tenant_a` user; call `GET/POST/PATCH/DELETE` and
   `/rpc/...` against rows belonging to `tenant_b`.
3. Assert every call is denied or returns zero rows — never tenant B's data.
4. Assert `tenant_a` **can** read/write its own rows (a false-positive guard —
   don't let an overly strict RLS policy pass the isolation test vacuously).

`cargo clippy --workspace -- -D warnings` and `cargo test --workspace` stay
green.
