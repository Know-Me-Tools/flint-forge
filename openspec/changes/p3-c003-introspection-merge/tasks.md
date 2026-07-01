# Tasks — p3-c003-introspection-merge

## Change
IntrospectionMerger: pg_graphql ∪ subscription SDL merge at __schema

## Status: PENDING (blocked on p3-c001 + p3-c007)

---

## Task List

### T1 — Add async-graphql dep to fdb-app
- [ ] In `crates/fdb-app/Cargo.toml`, add:
  ```toml
  async-graphql = { workspace = true }
  ```
- [ ] Run `cargo check -p fdb-app` — GREEN (dep addition only)

### T2 — Create IntrospectionMerger module
- [ ] Create `crates/fdb-app/src/graphql/` directory
- [ ] Create `crates/fdb-app/src/graphql/introspection.rs`
- [ ] Implement `is_introspection_query(body: &str) -> bool` (heuristic: `__schema` || `__type`)
- [ ] Implement `IntrospectionMerger::merge(pg_result: serde_json::Value, sub_schema: &Schema) -> serde_json::Value`:
  - Extract `data.__schema.types` from `pg_result`
  - Call `sub_schema.sdl()` to get subscription SDL string
  - Parse SDL to extract subscription type names
  - Append subscription types to `types[]` with dedup by `name` field (pg_graphql wins on conflict)
  - Set `data.__schema.subscriptionType = { "name": "Subscription" }` if absent
  - Return merged JSON value
- [ ] Add `pub mod graphql;` and `pub use graphql::introspection::IntrospectionMerger;` to `fdb-app/src/lib.rs`

### T3 — Wire introspection detection into handle_graphql_query
- [ ] In `crates/fdb-gateway/src/main.rs`, in `handle_graphql_query()`:
  - Add import: `use fdb_app::IntrospectionMerger;`
  - After extracting `body: String`, call `is_introspection_query(&body)`
  - If introspection: acquire `compiled = state.state_manager.current()`
    - If `compiled.subscription_schema.is_some()`: run pg_graphql then merge
    - If `None`: fall through to pg_graphql (subscription schema not yet compiled)
  - Non-introspection queries: no change in path

### T4 — Unit test: IntrospectionMerger
- [ ] In `crates/fdb-app/src/graphql/introspection.rs` `#[cfg(test)]`:
  - `test_introspection_merge_adds_subscription_types`:
    - Construct a minimal `serde_json::Value` simulating pg_graphql `__schema` output:
      ```json
      { "data": { "__schema": { "types": [{"name":"Query", "kind":"OBJECT"}], "subscriptionType": null } } }
      ```
    - Build a minimal async-graphql dynamic Schema with a Subscription type
    - Call `IntrospectionMerger::merge(pg_result, &schema)`
    - Assert `data.__schema.subscriptionType.name == "Subscription"`
    - Assert subscription type names appear in `data.__schema.types[]`
  - `test_introspection_merge_dedup_pg_wins`:
    - If both sources define type "Query", pg_graphql version is kept, subscription version dropped
- [ ] Run `cargo test -p fdb-app` — pass

### T5 — Compile and lint gate
- [ ] `cargo check --workspace` — GREEN
- [ ] `cargo clippy --workspace -- -D warnings` — GREEN
- [ ] `cargo test --workspace` — all tests pass
- [ ] Mark `p3-c003` as `qa_passed` in `progress.json`
