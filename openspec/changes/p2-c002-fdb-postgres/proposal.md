# p2-c002 — fdb-postgres: deadpool-postgres Pool + SET LOCAL RLS Context

## Change ID
`p2-c002-fdb-postgres`

## Phase
`p2-quarry-reflection-engine`

## Priority
P0 — MVP blocker

## Problem Statement

`PgBackend::acquire()` is a `todo!()` stub. No actual Postgres connection pool
exists. The three `SET LOCAL` statements required to propagate `RlsContext` into
Postgres session state — so that `flint_auth` helpers and row-level security
policies operate correctly — are entirely unimplemented.

Every database operation in Phase 2 is blocked on this change.

## Scope

### In Scope
- Add `deadpool-postgres` connection pool to `PgBackend`
- Initialize pool from environment variables at `PgBackend::new()`
- Implement `PgBackend::acquire(rls: &RlsContext) -> Result<Conn, BackendError>`
- Begin a `tokio_postgres::Transaction` on each acquired connection
- Execute the three `SET LOCAL` statements inside that transaction
- `Conn` wrapper type owning the open `Transaction` for its lifetime
- `thiserror`-based `BackendError` with `#[non_exhaustive]`

### Out of Scope
- GraphQL execution (Phase 3)
- Realtime stream adapter (Phase 3)
- Connection health monitoring beyond deadpool defaults

## Design

### Environment Variables

| Variable | Description |
|---|---|
| `DATABASE_URL` | `postgres://user:pass@host:port/db` |
| `DB_POOL_MAX_SIZE` | Max pool connections (default: 10) |
| `DB_POOL_TIMEOUT_SECS` | Checkout timeout (default: 30) |

### Conn Wrapper

The `Conn` wrapper must own the `tokio_postgres::Transaction` so that
`SET LOCAL` values do not escape into the next request's connection.
`SET LOCAL` only persists within an open transaction. When `Conn` drops,
the transaction is rolled back (correct behavior — user queries that need
commit must call `conn.commit()` explicitly).

```rust
// crates/fdb-postgres/src/conn.rs
use tokio_postgres::Transaction;

pub struct Conn<'c> {
    tx: Transaction<'c>,
}

impl<'c> Conn<'c> {
    pub async fn commit(self) -> Result<(), BackendError> {
        self.tx.commit().await.map_err(BackendError::Commit)
    }

    pub fn tx(&self) -> &Transaction<'_> {
        &self.tx
    }
}
```

### SET LOCAL Block

```rust
// crates/fdb-postgres/src/lib.rs
impl DatabaseBackend for PgBackend {
    async fn acquire(&self, rls: &RlsContext) -> Result<Conn<'_>, BackendError> {
        let client = self.pool
            .get()
            .await
            .map_err(BackendError::Pool)?;

        let tx = client
            .build_transaction()
            .isolation_level(IsolationLevel::ReadCommitted)
            .start()
            .await
            .map_err(BackendError::Begin)?;

        // These three SET LOCAL calls establish the RLS GUC context.
        // They MUST be inside the transaction — SET LOCAL does not persist
        // past the transaction boundary.
        // SECURITY: rls values are parameterized — never interpolated into SQL strings.
        tx.execute("SET LOCAL ROLE $1", &[&rls.role])
            .await
            .map_err(BackendError::SetLocal)?;
        tx.execute(
            r#"SET LOCAL "request.jwt.claims" = $1"#,
            &[&rls.claims_json],
        )
        .await
        .map_err(BackendError::SetLocal)?;
        let auth_header = format!("{{\"authorization\": \"Bearer {}\"}}", rls.raw_bearer);
        tx.execute(
            r#"SET LOCAL "request.headers" = $1"#,
            &[&auth_header],
        )
        .await
        .map_err(BackendError::SetLocal)?;

        Ok(Conn { tx })
    }
}
```

> **Security note on `raw_bearer` interpolation:** The `auth_header` string
> is constructed using `format!` but it is passed as a `$1` parameter to
> `tokio_postgres::Transaction::execute()` — the value is bound, not
> interpolated into the SQL statement itself. This is safe. The Postgres
> server receives the full string as a literal parameter value.

### PgBackend Initialization

```rust
pub struct PgBackend {
    pool: deadpool_postgres::Pool,
}

impl PgBackend {
    pub async fn from_env() -> Result<Self, BackendError> {
        let db_url = std::env::var("DATABASE_URL")
            .map_err(|_| BackendError::MissingEnv("DATABASE_URL"))?;
        let max_size = std::env::var("DB_POOL_MAX_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10usize);

        let pg_config: tokio_postgres::Config = db_url
            .parse()
            .map_err(|_| BackendError::InvalidConfig)?;

        let mgr_config = deadpool_postgres::ManagerConfig {
            recycling_method: deadpool_postgres::RecyclingMethod::Fast,
        };
        let mgr = deadpool_postgres::Manager::from_config(
            pg_config,
            tokio_postgres::NoTls,
            mgr_config,
        );
        let pool = deadpool_postgres::Pool::builder(mgr)
            .max_size(max_size)
            .build()
            .map_err(|_| BackendError::PoolBuild)?;

        Ok(Self { pool })
    }
}
```

### BackendError

```rust
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("pool checkout failed")]
    Pool(#[source] deadpool_postgres::PoolError),
    #[error("transaction begin failed")]
    Begin(#[source] tokio_postgres::Error),
    #[error("SET LOCAL failed")]
    SetLocal(#[source] tokio_postgres::Error),
    #[error("transaction commit failed")]
    Commit(#[source] tokio_postgres::Error),
    #[error("pool build failed")]
    PoolBuild,
    #[error("invalid DATABASE_URL")]
    InvalidConfig,
    #[error("missing required env var: {0}")]
    MissingEnv(&'static str),
}
```

## Security Contracts (NON-NEGOTIABLE)

1. **`rls.raw_bearer` and `rls.claims_json` are NEVER logged** — passed only as
   bound parameters to `tokio_postgres::Transaction::execute()`
2. **`SET LOCAL` MUST be inside the transaction** — `Conn` owns the `Transaction`
   and rolls it back on drop; this prevents GUC leakage across requests
3. **Column names used in REST queries are validated against `DatabaseModel`**
   before use in ORDER BY or SELECT — NOT parameterized (Postgres doesn't
   support parameterized identifiers), but validated against allowlist

## Dependencies to Add

### `fdb-postgres/Cargo.toml`
```toml
deadpool-postgres = { workspace = true }
tokio-postgres = { workspace = true }
tokio = { workspace = true, features = ["full"] }
thiserror = { workspace = true }
tracing = { workspace = true }
```

### `[workspace.dependencies]` in root `Cargo.toml`
```toml
deadpool-postgres = "0.14"
tokio-postgres = "0.7"
```

## Files Affected

| File | Change |
|---|---|
| `crates/fdb-postgres/src/lib.rs` | Replace `todo!()` bodies; add `PgBackend::from_env()` |
| `crates/fdb-postgres/src/conn.rs` | NEW — `Conn` wrapper owning `Transaction` |
| `crates/fdb-postgres/src/error.rs` | NEW — `BackendError` |
| `crates/fdb-postgres/Cargo.toml` | Add `deadpool-postgres`, `tokio-postgres` |
| `Cargo.toml` | Add `deadpool-postgres`, `tokio-postgres` to workspace deps |

## Gate Criteria

- `cargo check --workspace` passes
- `cargo clippy --workspace -- -D warnings` passes
- Integration tests with real Postgres (not mocked):
  - `test_acquire_sets_rls_role` — `SHOW ROLE` inside tx returns correct value
  - `test_acquire_sets_jwt_claims` — `SHOW "request.jwt.claims"` matches input
  - `test_set_local_does_not_escape_tx` — value absent on a fresh acquire
  - `test_pool_checkout_timeout` — returns `BackendError::Pool` on exhaustion
- No `rls` field values appear in `tracing` spans
