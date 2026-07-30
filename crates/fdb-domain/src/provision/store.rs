//! Plan-store value types exchanged across the `SchemaProvisioner` port
//! (FFS-001 D-P1: the `provision_ledger` IS the plan store — a `planned` row
//! is persisted at plan time and consumed at apply time).

use serde::{Deserialize, Serialize};

use super::plan::{PlanHash, PlanId};
use super::spec::{Namespace, SchemaSpec};

/// The executable form of a plan handed to the adapter: identity, drift-guard
/// hash, target namespace, and the generated DDL text. Never caller-supplied
/// SQL — this is the output of `fdb_app::provision::generate` only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatedPlan {
    /// Ledger identity of the plan being applied.
    pub plan_id: PlanId,
    /// Canonical spec hash (recomputed and compared by the gateway before
    /// this struct is ever built — the 409 drift guard).
    pub hash: PlanHash,
    /// Target namespace.
    pub namespace: Namespace,
    /// The generated DDL to execute in one transaction.
    pub ddl: String,
}

/// Result of an apply.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AppliedPlan {
    /// `true` when a plan with the same hash was already applied (idempotent
    /// replay — nothing was executed this time).
    pub already_applied: bool,
}

/// The row persisted at plan time (`status = 'planned'`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedRecord {
    /// Minted plan identity.
    pub plan_id: PlanId,
    /// Canonical spec hash.
    pub hash: PlanHash,
    /// Target namespace.
    pub namespace: Namespace,
    /// The full parsed spec (stored so apply can re-plan for the drift
    /// guard).
    pub spec: SchemaSpec,
    /// The generated DDL at plan time (stored for review/audit; apply
    /// re-generates and refuses on hash mismatch).
    pub ddl: String,
}

/// A stored plan loaded by hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredPlan {
    /// Ledger identity.
    pub plan_id: PlanId,
    /// The spec as persisted at plan time.
    pub spec: SchemaSpec,
    /// Row creation time as Unix seconds (the 24h expiry is enforced by the
    /// gateway against this).
    pub created_at_epoch: i64,
    /// Ledger status: `planned`, `applied`, or `failed`.
    pub status: String,
}

/// One column row for `CREATE TABLE` synthesis (FFS-001 §4.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdlColumn {
    /// Column name.
    pub name: String,
    /// Live Postgres type name (canonical spelling, e.g. `timestamp with
    /// time zone`).
    pub sql_type: String,
    /// Whether the column allows NULL.
    pub nullable: bool,
    /// The default expression as Postgres renders it, if any.
    pub default: Option<String>,
    /// 1-based position of the column within the primary key, when it is
    /// part of one — preserves composite-key order, which is NOT table
    /// column order in general.
    pub pk_ordinal: Option<i32>,
}

/// Everything needed to synthesize a `CREATE TABLE` string for one existing
/// table (FFS-001 §4.4 response).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDdlInfo {
    /// Columns in database order.
    pub columns: Vec<DdlColumn>,
    /// `pg_class.relrowsecurity`.
    pub rls_enabled: bool,
    /// `pg_class.relforcerowsecurity`.
    pub rls_forced: bool,
}

/// Most recent apply outcome, for `GET /schema/v1/status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerSummary {
    /// Ledger identity.
    pub plan_id: PlanId,
    /// When it happened, Unix seconds.
    pub at_epoch: i64,
    /// `applied` or `failed`.
    pub status: String,
}
