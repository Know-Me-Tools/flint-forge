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
