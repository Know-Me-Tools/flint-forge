# p2-c003 — fdb-reflection: New Crate — DatabaseModel IR + ReflectionEngine

## Change ID
`p2-c003-flint-reflection-crate`

## Phase
`p2-quarry-reflection-engine`

## Priority
P0 — MVP blocker; all other Phase 2 changes depend on this crate

## Problem Statement

The `fdb-reflection` crate does not exist. There is no `DatabaseModel` IR, no
`CompiledState`, no `StateManager`, and no `ReflectionEngine`. This is the
largest single deliverable of Phase 2 — a new adapter crate that queries the
`flint_meta` schema populated by the Phase 1 pgrx extension, assembles a typed
IR of the live database structure, and exposes it to the REST and OpenAPI
compilers.

Until this crate exists, Quarry cannot introspect the database. REST routing,
schema hot-reload, and OpenAPI generation are all blocked.

## Scope

### In Scope
- New crate `crates/fdb-reflection/` registered in workspace
- `DatabaseModel` IR: `Table`, `Column`, `Relationship`, `FnMeta`, `ViewMeta`
- `EncryptedDek(Vec<u8>)` newtype — ciphertext only, no plaintext key material
- `CompiledState` struct with `ArcSwap`-compatible design
- `ReflectionEngine::reflect()` — queries `flint_meta.*` functions
- Compiler module stubs: `rest.rs`, `openapi.rs`, `graphql.rs` (stub), `mcp.rs` (stub)
- Pipeline passes: normalization, validation, permission analysis, endpoint generation
- `ReflectionError` with `thiserror`, `#[non_exhaustive]`
- Workspace registration in root `Cargo.toml`

### Out of Scope
- `StateManager::start_listener()` hot-reload loop (p2-c005 — depends on this crate)
- REST compiler implementation (p2-c004 — uses types from this crate)
- OpenAPI compiler implementation (p2-c007)
- GraphQL compiler (Phase 3)
- MCP compiler (Phase 7)

## Hexagonal Rule

`fdb-reflection` is an **adapter crate** (Layer 1.5). It MUST NOT import
`fdb-gateway` (interface layer). The dependency direction is:

```
fdb-gateway → fdb-reflection → fdb-ports / fdb-domain → forge-domain
```

This is enforced by `cargo check --workspace` (circular deps fail at build time).

## Directory Structure

```
crates/fdb-reflection/
├── Cargo.toml
└── src/
    ├── lib.rs              (pub re-exports)
    ├── model.rs            (DatabaseModel + all IR types)
    ├── compiled.rs         (CompiledState)
    ├── engine.rs           (ReflectionEngine::reflect())
    ├── error.rs            (ReflectionError)
    ├── passes/
    │   ├── mod.rs
    │   ├── normalization.rs
    │   ├── validation.rs
    │   ├── permission_analysis.rs
    │   └── endpoint_generation.rs
    └── compilers/
        ├── mod.rs
        ├── rest.rs         (Phase 2 implementation — see p2-c004)
        ├── openapi.rs      (Phase 2 implementation — see p2-c007)
        ├── graphql.rs      (Phase 3 stub: todo!())
        └── mcp.rs          (Phase 7 stub: todo!())
```

## Key Types

### DatabaseModel (model.rs)

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatabaseModel {
    pub tables: Vec<Table>,
    pub functions: Vec<FnMeta>,
    pub views: Vec<ViewMeta>,
    pub version: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Table {
    pub schema: String,
    pub name: String,
    pub columns: Vec<Column>,
    pub pk: Vec<String>,
    pub fk: Vec<ForeignKey>,
    pub rls_enabled: bool,
    pub vault_key: Option<EncryptedDek>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Column {
    pub name: String,
    pub pg_type: String,
    pub nullable: bool,
    pub default: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ForeignKey {
    pub from_col: String,
    pub to_schema: String,
    pub to_table: String,
    pub to_col: String,
}

/// Ciphertext-only DEK wrapper.
/// CRITICAL: Plaintext key material MUST NOT appear in DatabaseModel or CompiledState.
/// The plaintext is only available transiently during vault_kms unwrap operations.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EncryptedDek(pub Vec<u8>);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FnMeta {
    pub schema: String,
    pub name: String,
    pub args: Vec<ArgMeta>,
    pub return_type: String,
    pub security_definer: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArgMeta {
    pub name: String,
    pub pg_type: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ViewMeta {
    pub schema: String,
    pub name: String,
    pub columns: Vec<Column>,
    pub security_barrier: bool,
}
```

### CompiledState (compiled.rs)

```rust
use axum::Router;
use std::sync::Arc;

/// The compiled, queryable snapshot of all database-driven routing state.
/// Stored in ArcSwap<CompiledState> — old guards are released when
/// in-flight requests complete; new state is swapped in atomically.
///
/// SECURITY: vault_key fields in DatabaseModel contain EncryptedDek (ciphertext).
/// Plaintext key material is never stored here.
#[derive(Debug)]
pub struct CompiledState {
    pub version: u64,
    pub database_model: Arc<DatabaseModel>,
    // Router is not Clone; hold in Arc so CompiledState is Arc-cheaply-clonable
    pub router: Arc<Router>,
    pub openapi_doc: serde_json::Value,
    // Phase 7: mcp_tools
    // Phase 5: agui_descriptors
}
```

### ReflectionEngine (engine.rs)

```rust
use sqlx::PgPool;

pub struct ReflectionEngine {
    pool: PgPool,
}

impl ReflectionEngine {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Query flint_meta.* and assemble a DatabaseModel.
    /// Must be called with service_role credentials (bypasses RLS on meta tables).
    #[tracing::instrument(skip(self), err)]
    pub async fn reflect(&self) -> Result<DatabaseModel, ReflectionError> {
        let version = self.fetch_version().await?;
        let tables = self.fetch_tables().await?;
        let functions = self.fetch_functions().await?;
        let views = self.fetch_views().await?;
        Ok(DatabaseModel { tables, functions, views, version })
    }
}
```

#### Queries

All queries target `flint_meta` functions installed by Phase 1 (`ext-flint-meta`):

```sql
-- Tables
SELECT schema_name, table_name, rls_enabled FROM flint_meta.tables();

-- Columns (per table)
SELECT column_name, pg_type, is_nullable, column_default
FROM flint_meta.columns($1, $2);

-- Relationships
SELECT from_schema, from_table, from_col, to_schema, to_table, to_col
FROM flint_meta.relationships();

-- Functions
SELECT schema_name, fn_name, return_type, security_definer
FROM flint_meta.functions();

-- Version
SELECT version FROM flint_meta.version();
```

### ReflectionError (error.rs)

```rust
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ReflectionError {
    #[error("database query failed")]
    Query(#[from] sqlx::Error),
    #[error("model validation failed: {0}")]
    Validation(String),
    #[error("compiler error: {0}")]
    Compiler(String),
}
```

## Pipeline Passes

### normalization.rs
- Deduplicate column names within a table
- Normalize schema names (lowercase, strip quotes)
- Convert Postgres type aliases to canonical form (`int4` → `integer`, etc.)

### validation.rs
- Assert each table has at least one column
- Assert FK references point to known tables in the model
- Assert no column name is a SQL keyword reserved for injection (ORDER BY guard)
- Emit `ReflectionError::Validation` on failure

### permission_analysis.rs
- Check `rls_enabled` flag per table
- Warn (via `tracing::warn!`) if a table has no RLS and is exposed to `anon`
- Phase 2: warn only; Phase 4: Cedar policy check will block

### endpoint_generation.rs
- Given validated `DatabaseModel`, produce ordered list of endpoints:
  - `GET /<schema>/<table>` (list with filter)
  - `GET /<schema>/<table>?id=eq.X` (single row)
  - `POST /<schema>/<table>` (insert)
  - `PATCH /<schema>/<table>?id=eq.X` (update)
  - `DELETE /<schema>/<table>?id=eq.X` (delete)
  - `POST /rpc/<schema>/<fn_name>` (function call)
- Consumed by `compilers/rest.rs` in p2-c004

## Cargo.toml

```toml
[package]
name = "fdb-reflection"
version = "0.1.0"
edition = "2021"

[dependencies]
fdb-domain = { path = "../fdb-domain" }
fdb-ports  = { path = "../fdb-ports" }
forge-domain = { path = "../../forge-domain" }

sqlx       = { workspace = true }
axum       = { workspace = true }
arc-swap   = { workspace = true }
serde      = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
thiserror  = { workspace = true }
tracing    = { workspace = true }
tokio      = { workspace = true, features = ["full"] }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
```

## Files Affected

| File | Change |
|---|---|
| `crates/fdb-reflection/` | NEW CRATE — entire directory |
| `Cargo.toml` | Add `"crates/fdb-reflection"` to `[workspace] members` |
| `Cargo.toml` | Add `sqlx = { version = "0.8", features = ["postgres", "runtime-tokio", "uuid", "json"] }` to workspace deps |

## Gate Criteria

- `cargo check --workspace` passes (new crate registered and compiles)
- `cargo clippy --workspace -- -D warnings` passes
- `cargo test -p fdb-reflection` passes:
  - `test_model_round_trip` — `DatabaseModel` serializes/deserializes cleanly
  - `test_encrypted_dek_contains_no_plaintext` — assert `EncryptedDek` has no `plaintext` field
  - `test_normalization_deduplicates_columns`
  - `test_validation_rejects_empty_table`
  - `test_validation_rejects_unknown_fk_target`
- `fdb-gateway` does NOT appear in `fdb-reflection`'s dependency tree
  (`cargo tree -p fdb-reflection` must not mention `fdb-gateway`)
