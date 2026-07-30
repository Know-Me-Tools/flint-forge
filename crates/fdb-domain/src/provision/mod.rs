//! Schema-provisioning domain types (FFS-001, p17).
//!
//! The typed spec a `/schema/v1/plan` request deserializes into, the plan
//! types the generator produces, and the validation pass between them. No
//! SQL strings cross this boundary inbound — the closed grammar here IS the
//! injection defense (FFS-001 D1).

pub mod plan;
pub mod spec;
pub mod store;
pub mod validate;

pub use plan::{Operation, OperationKind, Plan, PlanHash, PlanId};
pub use spec::{ColumnSpec, ColumnType, IndexSpec, Namespace, SchemaSpec, TableSpec};
pub use store::{
    AppliedPlan, DdlColumn, LedgerSummary, PlannedRecord, StoredPlan, TableDdlInfo, ValidatedPlan,
};
pub use validate::{validate_spec, SpecError};
