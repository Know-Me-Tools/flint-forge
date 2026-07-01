# p2-c004 — RestCompiler: DatabaseModel → axum::Router

## Change ID
`p2-c004-rest-compiler`

## Phase
`p2-quarry-reflection-engine`

## Priority
P0 — MVP blocker

## Problem Statement

`compilers/rest.rs` in `fdb-reflection` is a stub (created in p2-c003 as
a module file). The `PgRest::execute()` implementation in `fdb-postgres` is
also `todo!()`. There is no mechanism to translate the `DatabaseModel` IR
into live Axum routes with parameterized SQL CRUD handlers.

The Phase 2 gate criterion — RLS-correct REST CRUD under a real flint-gate
JWT — cannot be met until this change is complete.

## Scope

### In Scope
- `RestCompiler::compile(model: &DatabaseModel) -> axum::Router` in `fdb-reflection/src/compilers/rest.rs`
- One route group per table: `GET`, `POST`, `PATCH`, `DELETE`
- `/rpc/:fn_name` route for Postgres function calls
- Filter parsing: `eq`, `neq`, `gt`, `gte`, `lt`, `lte`, `like`, `ilike`, `in`, `is`, `cs`, `cd`
- Range header pagination: `Range: items=0-24` → `LIMIT 25 OFFSET 0`
- Column name allowlist validation against `DatabaseModel` before ORDER BY / SELECT
- HTTP method → SQL verb mapping
- Each handler acquires a `Conn` from `PgBackend::acquire(rls)` — the `SET LOCAL` block runs before any user query

### Out of Scope
- GraphQL (Phase 3)
- Realtime subscriptions (Phase 3)
- Vector search RPC (p2-c006)
- Row-level response transformation (Phase 5)

## Design

### Parameterized SQL — Critical Security Requirement

Column names in `SELECT`, `ORDER BY`, and `WHERE` clauses cannot be bound
as `$1` parameters (Postgres does not support parameterized identifiers).
All column names used in SQL construction MUST be validated against
`DatabaseModel.tables[].columns` before interpolation. Unknown columns
return HTTP 400. This is the injection prevention mechanism.

```rust
fn validate_column(col: &str, table: &Table) -> Result<&str, RestError> {
    if table.columns.iter().any(|c| c.name == col) {
        Ok(col)
    } else {
        Err(RestError::UnknownColumn(col.to_string()))
    }
}
```

Values in `WHERE` predicates are ALWAYS bound as `$N` parameters.

### Filter Operator Mapping

| Operator | SQL Fragment |
|---|---|
| `eq` | `col = $N` |
| `neq` | `col != $N` |
| `gt` | `col > $N` |
| `gte` | `col >= $N` |
| `lt` | `col < $N` |
| `lte` | `col <= $N` |
| `like` | `col LIKE $N` |
| `ilike` | `col ILIKE $N` |
| `in` | `col = ANY($N)` (value parsed as CSV or JSON array) |
| `is` | `col IS NULL` / `col IS NOT NULL` |
| `cs` | `col @> $N` (jsonb contains) |
| `cd` | `col <@ $N` (jsonb contained by) |

### HTTP Method → SQL Mapping

```
GET    /<schema>/<table>          → SELECT * FROM schema.table WHERE ... LIMIT ... OFFSET ...
POST   /<schema>/<table>          → INSERT INTO schema.table (...) VALUES (...) RETURNING *
PATCH  /<schema>/<table>?id=eq.X  → UPDATE schema.table SET ... WHERE id = $1 RETURNING *
DELETE /<schema>/<table>?id=eq.X  → DELETE FROM schema.table WHERE id = $1 RETURNING *
POST   /rpc/<schema>/<fn>         → SELECT * FROM schema.fn(...)
```

### Route Construction

```rust
// fdb-reflection/src/compilers/rest.rs
use axum::{Router, routing::{get, post, patch, delete}};
use crate::model::DatabaseModel;

pub struct RestCompiler;

impl RestCompiler {
    pub fn compile(model: &DatabaseModel) -> Router {
        let mut router = Router::new();

        for table in &model.tables {
            let prefix = format!("/{}/{}", table.schema, table.name);
            let t = table.clone();

            router = router
                .route(
                    &prefix,
                    get(move |state, query, headers| {
                        handlers::select(state, t.clone(), query, headers)
                    })
                    .post(move |state, headers, body| {
                        handlers::insert(state, t.clone(), headers, body)
                    }),
                )
                .route(
                    &format!("{}/:id", prefix),
                    patch(move |state, path, headers, body| {
                        handlers::update(state, t.clone(), path, headers, body)
                    })
                    .delete(move |state, path, headers| {
                        handlers::delete(state, t.clone(), path, headers)
                    }),
                );
        }

        for func in &model.functions {
            let prefix = format!("/rpc/{}/{}", func.schema, func.name);
            let f = func.clone();
            router = router.route(
                &prefix,
                post(move |state, headers, body| {
                    handlers::rpc(state, f.clone(), headers, body)
                }),
            );
        }

        router
    }
}
```

### Handler Pattern (select example)

```rust
// fdb-reflection/src/compilers/handlers.rs
pub async fn select(
    State(backend): State<Arc<dyn DatabaseBackend>>,
    table: Table,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Json<Vec<serde_json::Value>>, RestError> {
    let rls = extract_rls(&headers)?;
    let conn = backend.acquire(&rls).await.map_err(RestError::Backend)?;

    let (sql, bind_values) = build_select_query(&table, &params)?;
    let rows = conn.tx()
        .query(&sql, &bind_values.iter().map(|v| v as &(dyn ToSql + Sync)).collect::<Vec<_>>())
        .await
        .map_err(RestError::Query)?;

    Ok(Json(rows_to_json(rows)))
}
```

### Pagination via Range Header

```
Range: items=0-24   →  LIMIT 25 OFFSET 0
Range: items=25-49  →  LIMIT 25 OFFSET 25
```

Response includes `Content-Range: items 0-24/1000` header.

Default when no Range header: `LIMIT 1000 OFFSET 0`.

### RestError

```rust
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum RestError {
    #[error("unknown column: {0}")]
    UnknownColumn(String),
    #[error("invalid filter operator: {0}")]
    InvalidOperator(String),
    #[error("backend error")]
    Backend(#[from] BackendError),
    #[error("database query failed")]
    Query(#[source] tokio_postgres::Error),
    #[error("missing or invalid authorization header")]
    Unauthorized,
    #[error("request body deserialization failed")]
    BodyParse(#[source] serde_json::Error),
}

impl IntoResponse for RestError {
    fn into_response(self) -> axum::response::Response {
        let (status, msg) = match &self {
            RestError::UnknownColumn(_) | RestError::InvalidOperator(_) => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            RestError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".to_string()),
            _ => {
                // SECURITY: Do not leak internal error details to clients
                tracing::error!(error = %self, "rest handler error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error".to_string())
            }
        };
        (status, msg).into_response()
    }
}
```

## Files Affected

| File | Change |
|---|---|
| `crates/fdb-reflection/src/compilers/rest.rs` | Implement `RestCompiler::compile()` |
| `crates/fdb-reflection/src/compilers/handlers.rs` | NEW — CRUD + RPC handler fns |
| `crates/fdb-reflection/src/compilers/mod.rs` | Export `rest`, `handlers` |
| `crates/fdb-postgres/src/lib.rs` | Implement `PgRest::execute()` delegating to compiled router |

## Gate Criteria

Integration tests in `crates/fdb-reflection/tests/rest_compiler.rs`:

- `test_rest_select_with_eq_filter` — `GET /public/items?name=eq.Alice` returns correct rows
- `test_rest_select_all_filter_operators` — all 12 operators produce valid SQL and return results
- `test_rest_insert_returns_row` — `POST /public/items` with JSON body inserts and returns row
- `test_rest_patch_by_id` — `PATCH /public/items/1` updates and returns row
- `test_rest_delete_by_id` — `DELETE /public/items/1` removes and returns deleted row
- `test_rest_unknown_column_returns_400` — `?unknown_col=eq.foo` returns HTTP 400
- `test_rest_rpc_call` — `POST /rpc/public/echo` with body calls function and returns result
- `test_rls_role_visible_in_handler` — `SHOW ROLE` inside handler returns role from JWT
- `cargo clippy --workspace -- -D warnings` passes
