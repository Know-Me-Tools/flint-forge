# p3-c007 — GraphQlCompiler: DatabaseModel → async_graphql::Schema

## Change ID
`p3-c007-graphql-compiler`

## Phase
`p3-graphql-hybrid-engine`

## Priority
P0 — Required before p3-c004 (WebSocket) and p3-c003 (introspection merge)

## Problem Statement

`GraphQlCompiler::compile()` is `todo!("Phase 3: GraphQlCompiler")`. The
subscription schema (the "sibling" schema per §3.6) is never built. `CompiledState`
has no `subscription_schema` field. `StateManager::do_compile()` does not call
`GraphQlCompiler`. `async-graphql` is not in any `Cargo.toml`.

Without this, subscriptions and introspection merge are impossible.

## Scope

### In Scope
- Add `async-graphql = { version = "7", features = ["dynamic-schema"] }` to workspace `[workspace.dependencies]`
- Add `async-graphql-axum = "7"` to workspace deps (needed by p3-c004 but declared here)
- Add both deps to `fdb-reflection/Cargo.toml` and `fdb-gateway/Cargo.toml`
- Add `subscription_schema: Option<async_graphql::dynamic::Schema>` to `CompiledState`
- Implement `GraphQlCompiler::compile(model: &DatabaseModel) -> async_graphql::dynamic::Schema`:
  - For each `Table` in `model.tables`: generate a `tChanges` subscription field
    using async-graphql dynamic API
  - Subscription field: `<TableName>Changes(filter: <TableName>Filter): <TableName>ChangePayload!`
  - The dynamic schema covers ONLY subscriptions — not Query or Mutation (those go via pg_graphql)
- Wire `GraphQlCompiler::compile()` into `StateManager::do_compile()`
- Store the result in `CompiledState.subscription_schema`

### Out of Scope
- The WebSocket handler that serves the schema (p3-c004)
- Introspection merge with pg_graphql (p3-c003)
- Query or Mutation dynamic fields (these go via graphql.resolve() — p3-c001)

## Design

### Cargo.toml (workspace)

```toml
async-graphql = { version = "7", features = ["dynamic-schema"] }
async-graphql-axum = "7"
```

### CompiledState (`fdb-reflection/src/compiled.rs`)

```rust
pub struct CompiledState {
    pub version: u64,
    pub database_model: Arc<DatabaseModel>,
    pub router: Arc<Router>,
    pub openapi_doc: serde_json::Value,
    // Phase 3: subscription schema for async-graphql WebSocket handler
    pub subscription_schema: Option<async_graphql::dynamic::Schema>,
}
```

`subscription_schema` is `Option` because the first compile may produce an empty
schema if there are no tables yet — the handler must guard on `Some`.

### GraphQlCompiler (`fdb-reflection/src/compilers/graphql.rs`)

```rust
use async_graphql::dynamic::{Field, FieldFuture, Object, Schema, Subscription, TypeRef};

pub struct GraphQlCompiler;

impl GraphQlCompiler {
    pub fn compile(model: &DatabaseModel) -> async_graphql::dynamic::Schema {
        let mut subscription = Subscription::new("Subscription");
        for table in &model.tables {
            let type_name = pascal_case(&table.name);
            let field_name = format!("{}Changes", camel_case(&table.name));
            // Each subscription field: tChanges → ChangePayload stream
            let field = Field::new(
                &field_name,
                TypeRef::named_nn(format!("{}ChangePayload", type_name)),
                |_ctx| FieldFuture::new(async { Ok(None::<async_graphql::Value>) }),
            );
            subscription = subscription.field(field);
        }
        Schema::build("Query", None, Some("Subscription"))
            .register(subscription)
            // TODO(p3-c003): register ChangePayload and Filter types per table
            .finish()
            .unwrap_or_else(|_| Schema::build("Query", None, None).finish().expect("empty schema"))
    }
}
```

The `unwrap_or_else` fallback prevents a schema build error from crashing the
hot-reload loop — the previous `CompiledState` continues serving.

### StateManager::do_compile() (`fdb-reflection/src/state_manager.rs`)

```rust
async fn do_compile(engine: &ReflectionEngine) -> Result<CompiledState, ReflectionError> {
    let model = engine.reflect().await?;
    let router = RestCompiler::compile(&model);
    let openapi_doc = OpenApiCompiler::compile(&model);
    let subscription_schema = Some(GraphQlCompiler::compile(&model));
    Ok(CompiledState {
        version: model.version,
        database_model: Arc::new(model),
        router: Arc::new(router),
        openapi_doc,
        subscription_schema,
    })
}
```

## Security Contracts
- Schema generation MUST NOT embed raw JWT data or tenant IDs in type names
- Table names used as type names MUST be from the reflected `DatabaseModel` (whitelist) — not from request input
- `#[instrument(skip(model))]` on `GraphQlCompiler::compile()` to avoid logging model contents

## Acceptance Criteria
- `async-graphql = "7"` and `async-graphql-axum = "7"` in workspace `Cargo.toml`
- `CompiledState.subscription_schema` field exists with correct type
- `GraphQlCompiler::compile()` returns a valid `async_graphql::dynamic::Schema`
- `StateManager::do_compile()` calls `GraphQlCompiler::compile()` and stores the result
- Unit test `test_graphql_compiler_generates_subscription_schema_for_minimal_model` passes
- `cargo check --workspace` GREEN; clippy pedantic passes
