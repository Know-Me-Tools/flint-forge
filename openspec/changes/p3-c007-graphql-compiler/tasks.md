# Tasks — p3-c007-graphql-compiler

## Change
GraphQlCompiler: DatabaseModel → async_graphql::dynamic::Schema + CompiledState field

## Status: PENDING (blocked on p3-c001)

---

## Task List

### T1 — Add async-graphql to workspace dependencies
- [ ] In root `Cargo.toml` `[workspace.dependencies]`, add:
  ```toml
  async-graphql = { version = "7", features = ["dynamic-schema"] }
  async-graphql-axum = "7"
  ```
- [ ] Run `cargo check --workspace` — GREEN (dep addition only, no code yet)

### T2 — Add async-graphql to fdb-reflection Cargo.toml
- [ ] In `crates/fdb-reflection/Cargo.toml`, add:
  ```toml
  async-graphql = { workspace = true }
  ```

### T3 — Add subscription_schema field to CompiledState
- [ ] In `crates/fdb-reflection/src/compiled.rs`, add `use async_graphql::dynamic::Schema;`
- [ ] Add `pub subscription_schema: Option<Schema>` to `CompiledState` struct
- [ ] Update `CompiledState::fmt` (Debug impl) to include `subscription_schema` field indicator
- [ ] Fix all struct literal construction sites (only `state_manager.rs::do_compile()`) to add the new field

### T4 — Implement GraphQlCompiler::compile()
- [ ] In `crates/fdb-reflection/src/compilers/graphql.rs`, replace `todo!()` with:
  - `use async_graphql::dynamic::{Field, FieldFuture, Object, Schema, Subscription, TypeRef};`
  - Helper: `fn pascal_case(s: &str) -> String` — converts snake_case table name to PascalCase
  - Helper: `fn camel_case(s: &str) -> String` — converts snake_case to camelCase (for field names)
  - Build `Subscription` object with one field per table: `<TableName>Changes`
  - Each field type: `TypeRef::named_nn("<TableName>ChangePayload")`
  - Schema build: `Schema::build("Query", None, Some("Subscription")).register(subscription).finish()`
  - Fallback to empty schema on build error (prevents hot-reload loop crash)
- [ ] Add `#[allow(clippy::unused_async)]` if async-graphql FieldFuture requires async context

### T5 — Wire GraphQlCompiler into StateManager::do_compile()
- [ ] In `crates/fdb-reflection/src/state_manager.rs`:
  - Add `use crate::compilers::graphql::GraphQlCompiler;`
  - In `do_compile()`: add `let subscription_schema = Some(GraphQlCompiler::compile(&model));`
  - Add `subscription_schema` to `CompiledState { ... }` construction

### T6 — Unit test for GraphQlCompiler
- [ ] In `crates/fdb-reflection/src/compilers/graphql.rs` `#[cfg(test)]` module:
  - `test_graphql_compiler_generates_subscription_schema_for_minimal_model`:
    - Build a `DatabaseModel` with one table `("public", "items")`
    - Call `GraphQlCompiler::compile(&model)`
    - Assert the schema does not panic and schema type names include `itemsChanges` or `ItemsChanges`

### T7 — Add async-graphql-axum to fdb-gateway Cargo.toml (setup for p3-c004)
- [ ] In `crates/fdb-gateway/Cargo.toml`, add:
  ```toml
  async-graphql = { workspace = true }
  async-graphql-axum = { workspace = true }
  ```
  (No code uses them yet — just declaring the dependency for p3-c004)

### T8 — Compile and lint gate
- [ ] `cargo check --workspace` — GREEN
- [ ] `cargo clippy --workspace -- -D warnings` — GREEN
- [ ] `cargo test --workspace` — all existing tests + T6 test pass
- [ ] Mark `p3-c007` as `qa_passed` in `progress.json`
