# Tasks — p2-c006-pgvector-rpc

## Change
pgvector RPC: vector similarity search via /rpc path

## Status: DONE
## Priority: P1 — post-MVP
## Depends On: p2-c004 (REST compiler + RPC handler), p2-c003 (FnMeta type)

---

## Task List

### T0 — Prerequisites check (OQ-9)
- [x] Connect to Postgres 18 test container and run:
  `SELECT extversion FROM pg_extension WHERE extname = 'vector';`
- [x] If version < 0.7.0: update `images/postgres18/Dockerfile` to install pgvector from source at tag `v0.7.0` or later
- [x] Document pgvector version used in `docs/operations/pgvector-version.md`

### T1 — Add pgvector dependency
- [x] Add `pgvector = { version = "0.4", features = ["sqlx"] }` to `fdb-reflection/Cargo.toml`
- [x] Add `pgvector = "0.4"` to `[workspace.dependencies]` (if shared by other crates)
- [x] Verify `cargo check -p fdb-reflection` passes

### T2 — Update `ReflectionEngine::reflect()` to detect vector types
- [x] In `fetch_functions()`: detect `pg_type LIKE 'vector%'` in arg type from `flint_meta.functions()`
- [x] Store `ArgMeta.pg_type` as `"vector(N)"` for dimension-typed vectors
- [x] Add test: `test_reflect_detects_vector_arg_type` — reflection of test function with `vector(3)` arg returns correct `pg_type`

### T3 — Update RPC handler for vector args
- [x] In `handlers::rpc()`: check each `ArgMeta.pg_type.starts_with("vector")`
- [x] If vector: deserialize JSON `[f32, ...]` array → `pgvector::Vector`
- [x] Bind as `pgvector::Vector` typed param to the query
- [x] Otherwise: use existing `json_to_pg_param()` path

### T4 — Update RPC result serialization for vector columns
- [x] After `conn.tx().query()`, check each result column type
- [x] If column type is vector: call `.get::<pgvector::Vector>()` and serialize to `[f32, ...]` JSON array
- [x] Ensure non-vector columns still serialize via existing `rows_to_json()` path

### T5 — Integration tests `tests/pgvector_rpc.rs`
- [x] Create a test Postgres function `public.nearest_neighbors(query_vec vector(3))` that returns rows
- [x] `test_rpc_vector_arg_binds_correctly` — `POST /rpc/public/nearest_neighbors` with `{"query_vec": [0.1, 0.2, 0.3]}`
- [x] `test_rpc_vector_result_serializes_as_float_array` — result `vector` column returns `[f32, ...]`
- [x] `test_rpc_unknown_arg_type_returns_400` — arg with unsupported type returns 400
- [x] `test_pgvector_extension_version_gte_0_7_0` — assert extension version meets minimum (OQ-9 regression gate)
- [x] Mark tests `#[ignore]` if no `DATABASE_URL` env var

### T6 — Final verification
- [x] `cargo test -p fdb-reflection -- pgvector` passes (with `DATABASE_URL` set)
- [x] `cargo clippy --workspace -- -D warnings` — no warnings
- [x] `cargo check --workspace` — clean build
