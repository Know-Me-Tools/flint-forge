# Tasks — p2-c003-flint-reflection-crate

## Change
New crate `fdb-reflection` — DatabaseModel IR + ReflectionEngine

## Status: PENDING

---

## Task List

### T1 — Add workspace dependencies
- [ ] Add `sqlx = { version = "0.8", features = ["postgres", "runtime-tokio", "uuid", "json"] }` to `[workspace.dependencies]`
- [ ] Verify `cargo check --workspace` passes after dep addition

### T2 — Register new crate in workspace
- [ ] Add `"crates/fdb-reflection"` to `[workspace] members` in root `Cargo.toml`
- [ ] Verify `cargo check --workspace` still passes (crate doesn't exist yet — expect "not found" error, address in T3)

### T3 — Create crate skeleton
- [ ] Create `crates/fdb-reflection/Cargo.toml` with package metadata and deps:
  - `fdb-domain`, `fdb-ports`, `forge-domain` (path deps)
  - `sqlx`, `axum`, `arc-swap`, `serde/derive`, `serde_json`, `thiserror`, `tracing`, `tokio/full`
- [ ] Create `crates/fdb-reflection/src/lib.rs` with pub re-exports (empty for now)
- [ ] Verify `cargo check --workspace` passes with empty crate

### T4 — Create `src/error.rs`
- [ ] Define `ReflectionError` with `thiserror` and `#[non_exhaustive]`
- [ ] Variants: `Query(#[from] sqlx::Error)`, `Validation(String)`, `Compiler(String)`
- [ ] Export from `lib.rs`

### T5 — Create `src/model.rs`
- [ ] Define `EncryptedDek(Vec<u8>)` newtype — doc comment: "Ciphertext only. MUST NOT contain plaintext key material."
- [ ] Define `Column`, `ForeignKey`, `Table`, `FnMeta`, `ArgMeta`, `ViewMeta`, `DatabaseModel`
- [ ] All types: `#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]`
- [ ] Verify no field named `plaintext_dek`, `dek`, `key`, `secret`, or similar exists on any type
- [ ] Export all types from `lib.rs`

### T6 — Create `src/compiled.rs`
- [ ] Define `CompiledState` with fields: `version: u64`, `database_model: Arc<DatabaseModel>`,
  `router: Arc<axum::Router>`, `openapi_doc: serde_json::Value`
- [ ] Add doc comment: "SECURITY: vault_key fields in DatabaseModel contain EncryptedDek (ciphertext). Plaintext DEK is never stored here."
- [ ] Export from `lib.rs`

### T7 — Create `src/engine.rs`
- [ ] Define `ReflectionEngine { pool: sqlx::PgPool }`
- [ ] Implement `ReflectionEngine::new(pool: PgPool) -> Self`
- [ ] Implement `reflect() -> Result<DatabaseModel, ReflectionError>` calling:
  - `fetch_version()` → `SELECT version FROM flint_meta.version()`
  - `fetch_tables()` → `SELECT * FROM flint_meta.tables()`
  - For each table: `fetch_columns(schema, table)` → `SELECT * FROM flint_meta.columns($1, $2)`
  - `fetch_relationships()` → `SELECT * FROM flint_meta.relationships()`
  - `fetch_functions()` → `SELECT * FROM flint_meta.functions()`
- [ ] Add `#[tracing::instrument(skip(self), err)]` on `reflect()`
- [ ] Export from `lib.rs`

### T8 — Create passes module
- [ ] Create `src/passes/mod.rs` — declare sub-modules
- [ ] `normalization.rs`: deduplicate column names, normalize schema names, canonicalize PG types
- [ ] `validation.rs`: assert each table has ≥1 column; assert FK targets exist in model; assert no SQL keyword injection in column names
- [ ] `permission_analysis.rs`: `tracing::warn!` if table has no RLS and is exposed to `anon`
- [ ] `endpoint_generation.rs`: produce ordered endpoint list from `DatabaseModel`
- [ ] Wire passes into `ReflectionEngine::reflect()`: normalize → validate → permission_analysis

### T9 — Create compiler module stubs
- [ ] Create `src/compilers/mod.rs`
- [ ] `rest.rs`: `pub struct RestCompiler;` — `compile()` body is `todo!()` (implemented in p2-c004)
- [ ] `openapi.rs`: `pub struct OpenApiCompiler;` — `compile()` body is `todo!()` (implemented in p2-c007)
- [ ] `graphql.rs`: `pub struct GraphQlCompiler;` — body `todo!("Phase 3")`
- [ ] `mcp.rs`: `pub struct McpCompiler;` — body `todo!("Phase 7")`

### T10 — Unit tests
- [ ] `test_model_round_trip` — serialize and deserialize `DatabaseModel` round-trip cleanly
- [ ] `test_encrypted_dek_contains_no_plaintext` — static assert: no field named `plaintext*` on `Table`/`DatabaseModel` (use `serde_json::to_value` and check keys)
- [ ] `test_normalization_deduplicates_columns`
- [ ] `test_validation_rejects_empty_table`
- [ ] `test_validation_rejects_unknown_fk_target`

### T11 — Hexagonal rule verification
- [ ] Run `cargo tree -p fdb-reflection` and confirm `fdb-gateway` does NOT appear
- [ ] If it does appear, find the import chain and remove it (compile error expected)

### T12 — Final verification
- [ ] `cargo test -p fdb-reflection` — all unit tests pass
- [ ] `cargo clippy --workspace -- -D warnings` — no warnings
- [ ] `cargo check --workspace` — clean build
