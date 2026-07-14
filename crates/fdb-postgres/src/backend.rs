//! `PgBackend` — deadpool-backed Postgres connection pool implementing `DatabaseBackend`.

use async_trait::async_trait;
use deadpool_postgres::{Config as PoolConfig, Pool, Runtime};
use fdb_ports::{BackendError, Conn, DatabaseBackend};
use forge_identity::RlsContext;
use tracing::instrument;

use crate::conn::PgConn;
use crate::error::PgError;

/// Deadpool-backed Postgres connection pool implementing `DatabaseBackend`.
pub struct PgBackend {
    pub(crate) pool: Pool,
}

impl PgBackend {
    /// Build from `DATABASE_URL` environment variable.
    ///
    /// Expects a standard Postgres connection URL:
    /// `postgres://user:password@host:port/dbname`
    pub fn from_env() -> Result<Self, PgError> {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| PgError::Config("DATABASE_URL must be set".into()))?;

        let mut cfg = PoolConfig::new();
        cfg.url = Some(database_url);

        let pool = cfg
            .create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)
            .map_err(|e| PgError::Config(e.to_string()))?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl DatabaseBackend for PgBackend {
    /// Check out a connection from the pool, open a transaction, and apply all
    /// six `SET LOCAL` statements that propagate JWT context to Postgres RLS
    /// and the extended GUC layer (all within the same `BEGIN` transaction):
    ///
    /// ```sql
    /// SET LOCAL ROLE <role>;
    /// SET LOCAL "request.jwt.claims" = '<claims_json>';
    /// SET LOCAL "request.headers"    = '{"authorization":"Bearer <raw_bearer>"}';
    /// SET LOCAL "app.jwt_claims"     = '<claims_json>';
    /// SET LOCAL "app.keto_subject"   = '<keto_subject>';
    /// SET LOCAL "app.vault_key_id"   = '<vault_key_id>';
    /// ```
    ///
    /// The returned `Conn` keeps the connection alive for the duration of the
    /// caller's use. Callers MUST NOT log the `RlsContext` — it contains the
    /// raw bearer token and subject identifiers.
    #[instrument(skip(self, rls), fields(role = %rls.role), err)]
    async fn acquire(&self, rls: &RlsContext) -> Result<Conn, BackendError> {
        let object = self
            .pool
            .get()
            .await
            .map_err(|e| PgError::Checkout(e.to_string()))?;

        // SET LOCAL requires an open transaction.
        object
            .execute("BEGIN", &[])
            .await
            .map_err(PgError::Transaction)?;

        // ROLE: `SET` statements do NOT accept bind parameters, so the role reaches
        // SQL as an identifier. It is user-derived, so validate it first (fail closed)
        // and quote it — never interpolate an unvalidated value.
        if !forge_domain::is_safe_identifier(&rls.role) {
            return Err(PgError::SetLocal(format!("unsafe role identifier: {}", rls.role)).into());
        }
        object
            .execute(&format!(r#"SET LOCAL ROLE "{}""#, rls.role), &[])
            .await
            .map_err(|e| PgError::SetLocal(format!("SET LOCAL ROLE: {e}")))?;

        // The six GUCs: `SET LOCAL <name> = $1` is also invalid (SET rejects binds).
        // Use `set_config(name, value, is_local=true)`, which binds the VALUE safely
        // (the name is a fixed literal, never user input). Equivalent to SET LOCAL.
        let vault_key_id = rls.vault_key_id.as_deref().unwrap_or("");
        let headers_json = format!(r#"{{"authorization":"Bearer {}"}}"#, rls.raw_bearer);
        // (name, value) pairs — names are hardcoded, values are bound.
        let gucs: [(&str, &str); 5] = [
            ("request.jwt.claims", &rls.claims_json),
            ("request.headers", &headers_json),
            ("app.jwt_claims", &rls.claims_json),
            ("app.keto_subject", &rls.keto_subject),
            ("app.vault_key_id", vault_key_id),
        ];
        for (name, value) in gucs {
            object
                .execute("SELECT set_config($1, $2, true)", &[&name, &value])
                .await
                .map_err(|e| PgError::SetLocal(format!("set_config({name}): {e}")))?;
        }

        Ok(Conn(Box::new(PgConn::new(object))))
    }
}
