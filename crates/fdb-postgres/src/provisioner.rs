//! `PgProvisioner` — the `SchemaProvisioner` adapter (FFS-001 §8 Phase 2).
//!
//! Owns its **own** deadpool built from `PROVISIONER_DATABASE_URL` and its
//! own transaction discipline: explicit `BEGIN` → statements → `COMMIT` on
//! one connection. It deliberately does NOT go through
//! [`crate::PgBackend::acquire`] — that path opens a connection-lifetime
//! transaction whose uncommitted writes deadpool silently rolls back on
//! recycle (FFS-001 D7); for DDL that failure mode is a `200 OK` and no
//! table.
//!
//! DDL executes under `SET LOCAL ROLE flint_provisioner` (a constant, not
//! interpolated caller input), so created objects are owned by the dedicated
//! role and privilege containment is enforced by Postgres itself: `CREATE`
//! outside an operator-granted namespace fails with `42501`.
//!
//! Error mapping carries SQLSTATE only — never the rendered statement, which
//! may embed operator identifiers.

use async_trait::async_trait;
use deadpool_postgres::{Config as PoolConfig, Pool, Runtime};
use fdb_domain::provision::{
    AppliedPlan, LedgerSummary, Namespace, PlanHash, PlanId, PlannedRecord, StoredPlan,
    ValidatedPlan,
};
use fdb_domain::{ColumnMeta, TableMeta};
use fdb_ports::{BackendError, SchemaProvisioner};
use tracing::instrument;

use crate::error::PgError;

/// SQLSTATE for unique violations — a concurrent apply won the partial
/// unique index race on `(plan_hash) WHERE status='applied'`.
const UNIQUE_VIOLATION: &str = "23505";

/// The `SchemaProvisioner` Postgres adapter.
pub struct PgProvisioner {
    pool: Pool,
}

/// Map a `tokio_postgres::Error` to the port error, exposing SQLSTATE only.
fn sqlstate_only(op: &str, e: &tokio_postgres::Error) -> BackendError {
    let code = e
        .as_db_error()
        .map_or("none", |db| db.code().code());
    BackendError::Query(format!("provision {op} failed (SQLSTATE {code})"))
}

/// Extract the SQLSTATE code, if the error came from the server.
fn sqlstate_of(e: &tokio_postgres::Error) -> Option<String> {
    e.as_db_error().map(|db| db.code().code().to_owned())
}

impl PgProvisioner {
    /// Build from the `PROVISIONER_DATABASE_URL` environment variable.
    ///
    /// # Errors
    ///
    /// [`PgError::Config`] when the variable is unset/empty or deadpool
    /// rejects the connection string.
    pub fn from_env() -> Result<Self, PgError> {
        let url = std::env::var("PROVISIONER_DATABASE_URL")
            .ok()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| PgError::Config("PROVISIONER_DATABASE_URL must be set".into()))?;
        Self::from_url(&url)
    }

    /// Build from an explicit connection URL (tests, composition roots).
    ///
    /// # Errors
    ///
    /// [`PgError::Config`] when deadpool rejects the connection string.
    pub fn from_url(url: &str) -> Result<Self, PgError> {
        let mut cfg = PoolConfig::new();
        cfg.url = Some(url.to_owned());
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)
            .map_err(|e| PgError::Config(e.to_string()))?;
        Ok(Self { pool })
    }

    async fn conn(&self) -> Result<deadpool_postgres::Object, BackendError> {
        self.pool
            .get()
            .await
            .map_err(|e| PgError::Checkout(e.to_string()).into())
    }

    /// Best-effort `failed` ledger transition on its own connection, after
    /// the apply transaction rolled back. SQLSTATE only.
    async fn mark_failed(&self, plan_id: &PlanId, sqlstate: Option<&str>) {
        let Ok(conn) = self.conn().await else {
            tracing::warn!(plan_id = %plan_id, "could not record failed ledger row: no connection");
            return;
        };
        let result = conn
            .execute(
                "UPDATE flint_schema.provision_ledger \
                 SET status = 'failed', error_code = $2 \
                 WHERE plan_id = $1 AND status = 'planned'",
                &[&plan_id.as_str(), &sqlstate],
            )
            .await;
        if let Err(e) = result {
            tracing::warn!(plan_id = %plan_id, error = %sqlstate_only("mark-failed", &e), "failed ledger transition did not persist");
        }
    }
}

#[async_trait]
impl SchemaProvisioner for PgProvisioner {
    #[instrument(skip_all, fields(namespace = %ns))]
    async fn introspect_namespace(
        &self,
        ns: &Namespace,
    ) -> Result<Vec<TableMeta>, BackendError> {
        let conn = self.conn().await?;
        // pg_catalog is world-readable: no flint_meta grant is needed for the
        // provisioner role, and the query is bound, never interpolated.
        let rows = conn
            .query(
                "SELECT c.relname::text,
                        c.relrowsecurity,
                        a.attname::text,
                        format_type(a.atttypid, a.atttypmod),
                        NOT a.attnotnull
                 FROM pg_catalog.pg_class c
                 JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
                 JOIN pg_catalog.pg_attribute a
                   ON a.attrelid = c.oid AND a.attnum > 0 AND NOT a.attisdropped
                 WHERE n.nspname = $1 AND c.relkind = 'r'
                 ORDER BY c.relname, a.attnum",
                &[&ns.as_str()],
            )
            .await
            .map_err(|e| sqlstate_only("introspect", &e))?;

        let pk_rows = conn
            .query(
                "SELECT c.relname::text, a.attname::text
                 FROM pg_catalog.pg_index i
                 JOIN pg_catalog.pg_class c ON c.oid = i.indrelid
                 JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
                 JOIN pg_catalog.pg_attribute a
                   ON a.attrelid = c.oid AND a.attnum = ANY(i.indkey)
                 WHERE n.nspname = $1 AND i.indisprimary
                 ORDER BY c.relname, a.attnum",
                &[&ns.as_str()],
            )
            .await
            .map_err(|e| sqlstate_only("introspect-pk", &e))?;

        let mut tables: Vec<TableMeta> = Vec::new();
        for row in rows {
            let table: String = row.get(0);
            let rls: bool = row.get(1);
            let column = ColumnMeta {
                name: row.get(2),
                sql_type: row.get(3),
                nullable: row.get(4),
            };
            match tables.last_mut() {
                Some(t) if t.name == table => t.columns.push(column),
                _ => tables.push(TableMeta {
                    schema: ns.as_str().to_owned(),
                    name: table,
                    columns: vec![column],
                    primary_key: Vec::new(),
                    rls_enabled: rls,
                }),
            }
        }
        for row in pk_rows {
            let table: String = row.get(0);
            let col: String = row.get(1);
            if let Some(t) = tables.iter_mut().find(|t| t.name == table) {
                t.primary_key.push(col);
            }
        }
        Ok(tables)
    }

    #[instrument(skip_all, fields(plan_id = %record.plan_id, namespace = %record.namespace))]
    async fn persist_planned(&self, record: &PlannedRecord) -> Result<(), BackendError> {
        let conn = self.conn().await?;
        let spec_json = serde_json::to_value(&record.spec)
            .map_err(|e| BackendError::Query(format!("provision spec serialize: {e}")))?;
        conn.execute(
            "INSERT INTO flint_schema.provision_ledger \
             (plan_id, plan_hash, namespace, spec, generated_ddl, status) \
             VALUES ($1, $2, $3, $4, $5, 'planned') \
             ON CONFLICT (plan_id) DO NOTHING",
            &[
                &record.plan_id.as_str(),
                &record.hash.as_str(),
                &record.namespace.as_str(),
                &spec_json,
                &record.ddl,
            ],
        )
        .await
        .map_err(|e| sqlstate_only("persist-planned", &e))?;
        Ok(())
    }

    #[instrument(skip_all)]
    async fn load_planned(&self, hash: &PlanHash) -> Result<Option<StoredPlan>, BackendError> {
        let conn = self.conn().await?;
        let row = conn
            .query_opt(
                "SELECT plan_id, spec, extract(epoch FROM created_at)::bigint, status \
                 FROM flint_schema.provision_ledger \
                 WHERE plan_hash = $1 \
                 ORDER BY created_at DESC \
                 LIMIT 1",
                &[&hash.as_str()],
            )
            .await
            .map_err(|e| sqlstate_only("load-planned", &e))?;
        let Some(row) = row else { return Ok(None) };
        let spec_json: serde_json::Value = row.get(1);
        let spec = serde_json::from_value(spec_json)
            .map_err(|e| BackendError::Query(format!("provision spec deserialize: {e}")))?;
        Ok(Some(StoredPlan {
            plan_id: PlanId(row.get::<_, String>(0)),
            spec,
            created_at_epoch: row.get(2),
            status: row.get(3),
        }))
    }

    #[instrument(skip_all, fields(plan_id = %plan.plan_id, namespace = %plan.namespace))]
    async fn apply(
        &self,
        plan: &ValidatedPlan,
        applied_by: &str,
        version_before: Option<i64>,
    ) -> Result<AppliedPlan, BackendError> {
        let conn = self.conn().await?;

        let already = conn
            .query_opt(
                "SELECT 1 FROM flint_schema.provision_ledger \
                 WHERE plan_hash = $1 AND status = 'applied'",
                &[&plan.hash.as_str()],
            )
            .await
            .map_err(|e| sqlstate_only("apply-precheck", &e))?;
        if already.is_some() {
            return Ok(AppliedPlan {
                already_applied: true,
            });
        }

        // One explicit transaction on one connection: BEGIN → SET LOCAL ROLE
        // (constant identifier) → generated DDL → ledger transition → COMMIT.
        let result: Result<(), tokio_postgres::Error> = async {
            conn.batch_execute("BEGIN").await?;
            conn.batch_execute("SET LOCAL ROLE flint_provisioner").await?;
            conn.batch_execute(&plan.ddl).await?;
            conn.execute(
                "UPDATE flint_schema.provision_ledger \
                 SET status = 'applied', applied_by = $2, applied_at = now(), \
                     version_before = $3 \
                 WHERE plan_id = $1",
                &[&plan.plan_id.as_str(), &applied_by, &version_before],
            )
            .await?;
            conn.batch_execute("COMMIT").await?;
            Ok(())
        }
        .await;

        match result {
            Ok(()) => Ok(AppliedPlan {
                already_applied: false,
            }),
            Err(e) => {
                // Roll back whatever partial state the transaction holds; the
                // failed-ledger transition then happens on a fresh implicit
                // transaction so it survives the rollback.
                let _ = conn.batch_execute("ROLLBACK").await;
                let sqlstate = sqlstate_of(&e);
                if sqlstate.as_deref() == Some(UNIQUE_VIOLATION) {
                    // A concurrent apply of the same hash won the partial
                    // unique index; its DDL is committed, ours rolled back.
                    return Ok(AppliedPlan {
                        already_applied: true,
                    });
                }
                self.mark_failed(&plan.plan_id, sqlstate.as_deref()).await;
                Err(sqlstate_only("apply", &e))
            }
        }
    }

    #[instrument(skip_all)]
    async fn last_apply(&self) -> Result<Option<LedgerSummary>, BackendError> {
        let conn = self.conn().await?;
        let row = conn
            .query_opt(
                "SELECT plan_id, \
                        extract(epoch FROM coalesce(applied_at, created_at))::bigint, \
                        status \
                 FROM flint_schema.provision_ledger \
                 WHERE status IN ('applied', 'failed') \
                 ORDER BY coalesce(applied_at, created_at) DESC \
                 LIMIT 1",
                &[],
            )
            .await
            .map_err(|e| sqlstate_only("last-apply", &e))?;
        Ok(row.map(|row| LedgerSummary {
            plan_id: PlanId(row.get::<_, String>(0)),
            at_epoch: row.get(1),
            status: row.get(2),
        }))
    }
}
