# p2-c006 — pgvector RPC: Vector Similarity Search via /rpc

## Change ID
`p2-c006-pgvector-rpc`

## Phase
`p2-quarry-reflection-engine`

## Priority
P1 — post-MVP; does not block REST gate

## Problem Statement

Quarry exposes Postgres functions via `/rpc/<schema>/<fn_name>` (implemented in
p2-c004 as generic function dispatch). However, vector similarity search
functions using `pgvector` require special handling:
- Input vectors must be typed as `vector(N)` for Postgres to match the function signature
- `sqlx` does not have native `pgvector` type support — requires the `pgvector` crate
- The REST compiler's generic RPC handler cannot decode `float[]` JSON arrays into
  Postgres `vector` type without explicit type annotation

This change adds first-class `pgvector` type support to the Quarry RPC path so
that `flint_llm` embedding functions and custom vector search functions are
accessible via the API.

## Scope

### In Scope
- Add `pgvector` crate integration to `fdb-reflection` (or a new `fdb-vector` adapter)
- Detect `vector(N)` Postgres parameter type in `FnMeta.args` during reflection
- Special-case RPC handler: when arg type is `vector`, decode JSON `[f32, ...]` array
  and bind as `pgvector::Vector`
- Response: rows with `vector` columns serialized as JSON `[f32, ...]` arrays
- Integration test: `POST /rpc/public/match_documents` with embedding input

### Out of Scope
- Vector indexing / HNSW index management (operations concern)
- Embedding generation (delegated to `flint_llm` / liter-llm)
- Direct vector column exposure via REST (Phase 5 — requires special serialization)
- ANN index selection (Phase 5)

## Design

### pgvector Dependency

```toml
# fdb-reflection/Cargo.toml or fdb-vector/Cargo.toml
pgvector = { version = "0.4", features = ["sqlx"] }
```

### FnMeta Detection

During `ReflectionEngine::reflect()`, when `flint_meta.functions()` returns
a function argument with `pg_type LIKE 'vector%'`, mark `ArgMeta.pg_type`
as `"vector(N)"` where N is the dimension.

### RPC Handler Override

In `compilers/handlers.rs`, the RPC handler checks each arg:

```rust
for (arg_meta, json_value) in func.args.iter().zip(body_args.iter()) {
    if arg_meta.pg_type.starts_with("vector") {
        // Decode JSON array of f32 → pgvector::Vector
        let floats: Vec<f32> = serde_json::from_value(json_value.clone())
            .map_err(|_| RestError::BodyParse(...))?;
        let vec = pgvector::Vector::from(floats);
        params.push(Box::new(vec) as Box<dyn ToSql + Send + Sync>);
    } else {
        params.push(json_to_pg_param(json_value, &arg_meta.pg_type)?);
    }
}
```

### Response Serialization

Vector columns in result rows are serialized back to JSON `[f32, ...]` arrays
using `pgvector::Vector::to_vec()`.

### Open Question: OQ-9

**OQ-9:** Is `pgvector >= 0.7.0` available in the Phase 2 Postgres 18 Docker image?
The `flint_meta` schema is on PG18. `pgvector` 0.7.0 is required for HNSW index
support and PG18 compatibility. Check before executing this change:

```bash
docker exec <postgres-container> psql -U postgres -c "SELECT extversion FROM pg_extension WHERE extname = 'vector';"
```

If version < 0.7.0, update `images/postgres18/Dockerfile` to install pgvector
from source at the correct tag.

## Files Affected

| File | Change |
|---|---|
| `crates/fdb-reflection/src/compilers/handlers.rs` | Add vector arg detection + binding |
| `crates/fdb-reflection/src/engine.rs` | Detect `vector(N)` type in `FnMeta` during reflect |
| `crates/fdb-reflection/Cargo.toml` | Add `pgvector` dependency |
| `Cargo.toml` | Add `pgvector` to workspace deps if shared |

## Gate Criteria

Tests in `crates/fdb-reflection/tests/pgvector_rpc.rs`:

- `test_rpc_vector_arg_binds_correctly` — `POST /rpc/public/nearest_neighbors` with
  `{"embedding": [0.1, 0.2, 0.3]}` reaches a test function that accepts `vector(3)`
- `test_rpc_vector_result_serializes_as_float_array` — result row with `vector` column
  returns `[f32, ...]` JSON array
- `test_rpc_unknown_arg_type_returns_400` — unknown type returns HTTP 400 (not 500)
- OQ-9 check: add a test that verifies pgvector extension version >= 0.7.0 in test DB
