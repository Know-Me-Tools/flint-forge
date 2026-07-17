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
    ///
    /// # Errors
    ///
    /// Returns [`PgError::Config`] when the `DATABASE_URL` environment
    /// variable is unset, or when `deadpool_postgres` rejects the connection
    /// string / fails to build the pool.
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
    ///
    /// # Errors
    ///
    /// Returns `BackendError` when: the pool has no connections available
    /// (checkout failure); `rls.role` fails the safe-identifier check (fail
    /// closed rather than interpolate an unvalidated role into `SET LOCAL
    /// ROLE`); or any of the `BEGIN`/`SET LOCAL ROLE`/`set_config` statements
    /// is rejected by Postgres.
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

    /// Run a single SQL statement under the full RLS context and return each
    /// result row JSON-encoded (`{"column": value, ...}`).
    ///
    /// `params` are JSON-encoded scalar bind values, one per `$N` placeholder
    /// — the same "send raw text, let Postgres resolve the target type"
    /// binding strategy `RestExecutor` uses for untyped filter values, not a
    /// typed bind. `sql` may be a plain `SELECT` or a DML statement with
    /// `RETURNING`; a DML statement with no `RETURNING` succeeds with zero
    /// rows.
    ///
    /// Consumed by Flint Kiln's `flint:host/db` and `flint:host/llm` host
    /// implementations to forward a WASM component's governed SQL call
    /// through the same connection/RLS-context machinery REST and GraphQL
    /// already use — see `fke-runtime`.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError`] when the RLS-scoped connection cannot be
    /// acquired, or when Postgres rejects `sql`/`params` or execution
    /// otherwise fails.
    async fn query_json(
        &self,
        rls: &RlsContext,
        sql: &str,
        params: &[String],
    ) -> Result<Vec<String>, BackendError> {
        let conn = self.acquire(rls).await?;
        let pg_conn = PgConn::from_conn(&conn)
            .ok_or_else(|| BackendError::Internal("unexpected conn type in PgBackend".into()))?;

        let owned: Vec<Option<AnyText>> = params.iter().map(|p| json_param_to_bind(p)).collect();
        let binds: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = owned
            .iter()
            .map(|b| match b {
                Some(a) => a as &(dyn tokio_postgres::types::ToSql + Sync),
                None => &Option::<String>::None,
            })
            .collect();

        let wrapped = format!(
            "WITH __flint_kiln_query AS ({sql}) SELECT to_jsonb(t) FROM __flint_kiln_query t"
        );
        let rows = pg_conn
            .inner
            .query(&wrapped, &binds)
            .await
            .map_err(|e| BackendError::Query(format!("kiln query_json: {e}")))?;

        rows.iter()
            .map(|row| {
                let value: serde_json::Value = row
                    .try_get(0)
                    .map_err(|e| BackendError::Internal(format!("kiln query_json row: {e}")))?;
                Ok(value.to_string())
            })
            .collect()
    }
}

/// Decode a `flint:host/db` JSON-encoded parameter into an [`AnyText`] bind
/// value, or `None` for SQL `NULL` (JSON `null`).
///
/// Non-string JSON values (numbers, bools, arrays, objects) bind their
/// original JSON text verbatim — valid Postgres input-function text for both
/// scalar and `jsonb`-typed target columns. Malformed JSON is treated as a
/// literal string rather than rejected outright — the WIT contract always
/// JSON-encodes on the guest side, so this only matters defensively.
fn json_param_to_bind(raw: &str) -> Option<AnyText> {
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(serde_json::Value::Null) => None,
        Ok(serde_json::Value::String(s)) => Some(AnyText(s)),
        _ => Some(AnyText(raw.to_owned())),
    }
}

/// Wraps a JSON-decoded scalar as Postgres wire-format text, bound with
/// `accepts()` returning `true` for every type — Postgres resolves the target
/// column type from context and parses this via the type's input-function
/// syntax, only as a value literal via the target type's input function — a bad
/// literal (e.g. non-numeric text for `int4`) is rejected by Postgres, not
/// executed.
#[derive(Debug)]
struct AnyText(String);

impl tokio_postgres::types::ToSql for AnyText {
    fn to_sql(
        &self,
        _ty: &tokio_postgres::types::Type,
        out: &mut bytes::BytesMut,
    ) -> Result<tokio_postgres::types::IsNull, Box<dyn std::error::Error + Sync + Send>> {
        out.extend_from_slice(self.0.as_bytes());
        Ok(tokio_postgres::types::IsNull::No)
    }

    fn accepts(_ty: &tokio_postgres::types::Type) -> bool {
        true
    }

    fn encode_format(&self, _ty: &tokio_postgres::types::Type) -> tokio_postgres::types::Format {
        tokio_postgres::types::Format::Text
    }

    tokio_postgres::types::to_sql_checked!();
}
