//! Plan-side domain types for schema provisioning (FFS-001 §4.2 response
//! shape): the operations list, the plan hash, and the plan identity.

use serde::{Deserialize, Serialize};

use super::spec::Namespace;

/// Identifier of a persisted plan (`pln_…`).
///
/// Minted at persist time by the gateway — never by the pure generator, which
/// must stay deterministic (same spec ⇒ same output, including the hash).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct PlanId(pub String);

impl PlanId {
    /// The raw identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PlanId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Content hash of a canonicalized [`super::spec::SchemaSpec`]
/// (`sha256:<hex>`).
///
/// The apply drift guard recomputes this from the stored spec against the
/// live schema and refuses on mismatch (FFS-001 D2).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct PlanHash(pub String);

impl PlanHash {
    /// The raw `sha256:<hex>` string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PlanHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The kind of one generated operation (FFS-001 §4.2 `operations[].kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum OperationKind {
    /// `CREATE SCHEMA IF NOT EXISTS`
    CreateSchema,
    /// `CREATE TABLE`
    CreateTable,
    /// `ALTER TABLE … ADD COLUMN`
    AddColumn,
    /// `ALTER TABLE … ENABLE/FORCE ROW LEVEL SECURITY`
    EnableRls,
    /// One generated tenant policy
    CreatePolicy,
    /// `CREATE INDEX` (generated tenant index or caller-declared)
    CreateIndex,
    /// `GRANT … TO authenticated`
    Grant,
    /// `COMMENT ON TABLE`
    Comment,
}

/// One entry in a plan's operations list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    /// What the operation does.
    pub kind: OperationKind,
    /// The object it targets (qualified where meaningful, e.g.
    /// `sansaba_sourcing.permit_watch`).
    pub target: String,
    /// Whether the target already exists in the live schema (diff result).
    pub exists: bool,
}

/// The pure output of `generate(spec, live)`: everything the `/plan` response
/// needs except the persisted identity (`planId`, `expiresAt`), which the
/// gateway adds at persist time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    /// The namespace the plan targets.
    pub namespace: Namespace,
    /// Ordered operations the DDL will perform.
    pub operations: Vec<Operation>,
    /// The full generated DDL text, reviewable as-is.
    pub ddl: String,
    /// Non-fatal findings (e.g. an acknowledged unscoped table).
    pub warnings: Vec<String>,
    /// `true` when the live schema already satisfies the spec and the DDL is
    /// empty.
    pub noop: bool,
    /// Content hash of the canonicalized input spec.
    pub hash: PlanHash,
}
