//! `SchemaProvisioner` — the only DDL-adjacent seam (FFS-001 §7).
//!
//! Sits beside [`crate::SchemaProvider`], which *introspects*; this port
//! *mutates*. It applies generated, validated DDL and owns the plan-store
//! rows in `flint_schema.provision_ledger`. It never accepts caller-supplied
//! SQL: every `ValidatedPlan` is the output of `fdb_app::provision::generate`
//! over a spec that already passed the closed-grammar validation.

use async_trait::async_trait;
use fdb_domain::provision::{
    AppliedPlan, LedgerSummary, Namespace, PlanHash, PlannedRecord, StoredPlan, ValidatedPlan,
};
use fdb_domain::TableMeta;

use crate::BackendError;

/// Applies generated, validated DDL and persists the provisioning ledger.
///
/// Implementations MUST use their own connection pool with explicit
/// `BEGIN`/`COMMIT` discipline — never `DatabaseBackend::acquire`, whose
/// connection-lifetime transaction silently discards uncommitted writes on
/// recycle (FFS-001 D7) — and MUST execute as the dedicated
/// `flint_provisioner` role, not the caller's role and not the migration
/// owner (FFS-001 D3).
#[async_trait]
pub trait SchemaProvisioner: Send + Sync {
    /// Live table/column/RLS state for one namespace, as the generator's
    /// diff input.
    ///
    /// # Errors
    ///
    /// [`BackendError::Query`] carrying SQLSTATE context only — never a
    /// rendered statement.
    async fn introspect_namespace(&self, ns: &Namespace)
        -> Result<Vec<TableMeta>, BackendError>;

    /// Persist a `planned` ledger row (the durable plan store, FFS-001
    /// D-P1). One `flint_schema` row; no user-schema writes.
    ///
    /// # Errors
    ///
    /// [`BackendError::Query`] with SQLSTATE context on insert failure.
    async fn persist_planned(&self, record: &PlannedRecord) -> Result<(), BackendError>;

    /// Load the most recent stored plan for a hash, if any.
    ///
    /// # Errors
    ///
    /// [`BackendError::Query`] with SQLSTATE context on query failure.
    async fn load_planned(&self, hash: &PlanHash) -> Result<Option<StoredPlan>, BackendError>;

    /// Execute a validated plan in one transaction and commit; transition the
    /// ledger row to `applied` (or `failed` on error, in its own follow-up
    /// write after rollback).
    ///
    /// `applied_by` is the caller's JWT `sub` — attribution only, never the
    /// bearer, and implementations MUST NOT log it. `version_before` is the
    /// reflection schema version sampled by the gateway before apply.
    ///
    /// # Errors
    ///
    /// [`BackendError::Query`] carrying SQLSTATE only — never the rendered
    /// statement, which may embed operator identifiers.
    async fn apply(
        &self,
        plan: &ValidatedPlan,
        applied_by: &str,
        version_before: Option<i64>,
    ) -> Result<AppliedPlan, BackendError>;

    /// Stamp the reflection schema version observed after apply onto an
    /// `applied` ledger row. Separate from [`SchemaProvisioner::apply`]
    /// because the post-apply version is only observable once the (async)
    /// reflection recompile lands — the gateway samples it and reports back.
    ///
    /// # Errors
    ///
    /// [`BackendError::Query`] with SQLSTATE context on update failure.
    async fn record_version_after(
        &self,
        plan_id: &fdb_domain::provision::PlanId,
        version_after: i64,
    ) -> Result<(), BackendError>;

    /// The most recent `applied`/`failed` ledger row, for `/status`.
    ///
    /// # Errors
    ///
    /// [`BackendError::Query`] with SQLSTATE context on query failure.
    async fn last_apply(&self) -> Result<Option<LedgerSummary>, BackendError>;

    /// Column rows + RLS flags for one table, for `CREATE TABLE` synthesis
    /// (`GET /schema/v1/tables/{schema}/{table}/ddl`, FFS-001 §4.4).
    /// `None` when the table does not exist.
    ///
    /// # Errors
    ///
    /// [`BackendError::Query`] with SQLSTATE context on query failure.
    async fn table_ddl_info(
        &self,
        ns: &Namespace,
        table: &str,
    ) -> Result<Option<fdb_domain::provision::TableDdlInfo>, BackendError>;
}
