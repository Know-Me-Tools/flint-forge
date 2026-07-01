# p3-c001 — GraphQL Passthrough: POST /graphql → graphql.resolve()

## Change ID
`p3-c001-graphql-passthrough`

## Phase
`p3-graphql-hybrid-engine`

## Priority
P0 — MVP blocker; requires p3-c005 (OQ-3 resolved) and p3-c008 (instrument fix) first

## Problem Statement

`POST /graphql` does not exist in `fdb-gateway`. Query and Mutation operations
have nowhere to land. `PgGraphQl::execute()` is `todo!()` — the
`SELECT graphql.resolve($query, $variables, $extensions)` call is unwritten.

Per the spec (§3.2), Query and Mutation MUST go to `graphql.resolve()` under
full RLS context, with async-graphql NOT in this path. The response JSON from
pg_graphql is returned verbatim.

## Scope

### In Scope
- Register `POST /graphql` route in `fdb-gateway/src/main.rs`
- Implement `PgGraphQl::execute()` in `fdb-postgres/src/lib.rs`:
  ```sql
  SELECT graphql.resolve($1::text, $2::jsonb, $3::jsonb)
  ```
  where $1 = query, $2 = variables (default `{}`), $3 = extensions (default `{}`)
- JWT extraction from `Authorization: Bearer <token>` header in the handler
- Call `fdb-auth::rls_from_bearer()` → `RlsContext`
- Call `PgBackend::acquire(rls)` → `Conn` with 6 SET LOCAL statements
- Execute `graphql.resolve()` on the connection (inside the already-open transaction)
- Return the JSON result verbatim with `Content-Type: application/json`
- Error mapping: Postgres errors → `500`; auth errors → `401`
- Column-name injection guard: `GraphQlRequest` fields are bound as params via `$N` — NOT interpolated into SQL

### Out of Scope
- Subscriptions (p3-c004)
- Introspection merge (p3-c003)
- Detecting subscription vs. query/mutation (that split is in p3-c004)

## Design

### Handler (fdb-gateway/src/main.rs)

```rust
async fn handle_graphql_query(
    State(state): State<GatewayState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<GraphQlRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, &'static str)> {
    let bearer = extract_bearer(&headers)
        .ok_or((StatusCode::UNAUTHORIZED, "missing bearer token"))?;
    let rls = fdb_auth::rls_from_bearer(bearer)
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid token"))?;
    let executor = state.pg_graphql.clone();
    let result = executor.execute(req, &rls)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "query failed"))?;
    Ok(Json(result.0))
}
```

### PgGraphQl::execute() (fdb-postgres/src/lib.rs)

```rust
impl GraphQlExecutor for PgGraphQl {
    #[instrument(skip(self, rls), fields(role = %rls.role), err)]
    async fn execute(&self, req: GraphQlRequest, rls: &RlsContext) -> Result<Json, BackendError> {
        let backend = PgBackend { pool: self.pool.clone() };
        let conn = backend.acquire(rls).await?;
        let pg_conn = PgConn::from_conn(&conn)
            .ok_or(BackendError::Connection)?;

        let variables = req.variables
            .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null))
            .unwrap_or(serde_json::Value::Null);
        let extensions = serde_json::Value::Null;

        let row: (serde_json::Value,) = sqlx_like_query_on_pg_conn(
            "SELECT graphql.resolve($1::text, $2::jsonb, $3::jsonb)",
            &req.query, &variables, &extensions,
        ).await.map_err(|e| BackendError::Query(e.to_string()))?;

        Ok(Json(row.0))
    }
}
```

Note: `PgGraphQl` holds a `Pool` (deadpool-postgres). The `graphql.resolve()` 
call runs on a connection from `PgBackend::acquire()` that has already opened
the transaction and set all 6 GUC values. The result is returned verbatim.

### GatewayState extension

`GatewayState` must gain a `pg_graphql: Arc<PgGraphQl>` field. `PgGraphQl`
is constructed from the same `Pool` in `main()`.

## Security Contracts
- Handler MUST NOT log the bearer token, claims, or `rls.raw_bearer`
- `graphql.resolve()` arguments are ALWAYS bound as `$N` params — never string-interpolated
- Postgres connection has all 6 SET LOCAL statements set BEFORE `graphql.resolve()` is called
- Column names from `DatabaseModel` used in filter generation MUST be validated
  against the reflected schema (whitelist) — closes Phase 2 security debt item 2

## Acceptance Criteria
- `POST /graphql` route registered in `fdb-gateway`
- `PgGraphQl::execute()` calls `graphql.resolve()` with parameterized args
- `cargo check --workspace` GREEN
- `cargo clippy --workspace -- -D warnings` GREEN
- Unit test `test_pg_graphql_execute_builds_without_live_db` passes (mock or compile-only test)
- Integration test documented: `POST /graphql { "query": "{ __typename }" }` → 200 with JSON body
