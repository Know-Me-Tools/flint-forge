//! Typed schema-provisioning spec (FFS-001 §4.2 request shape).
//!
//! The closed grammar is the load-bearing security decision (FFS-001 D1): the
//! API deserializes into these types and *generates* SQL — it never accepts
//! SQL. An injection attempt fails here, at type-checking, before any code
//! runs: column types are a closed enum, and every identifier is validated by
//! [`crate::provision::validate`] before generation.

use serde::{Deserialize, Serialize};

/// A namespace (Postgres schema) targeted by a provisioning spec.
///
/// Plain wrapper over the raw name; validity (identifier safety, reserved
/// prefixes) is checked by [`crate::provision::validate::validate_spec`], not
/// at construction, so error reporting can name every problem in one pass.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Namespace(pub String);

impl Namespace {
    /// The raw schema name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Namespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Top-level provisioning request: one namespace, one or more tables.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SchemaSpec {
    /// Target Postgres schema. Must pass identifier validation and must not
    /// be a reserved namespace (`flint_*`, `public`, `pg_*`,
    /// `information_schema`).
    pub namespace: Namespace,
    /// Tables to declare. Additive-only: a table absent from the spec is
    /// never touched, and no destructive operation is ever generated.
    pub tables: Vec<TableSpec>,
}

/// One declared table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TableSpec {
    /// Unqualified table name.
    pub name: String,
    /// Optional human-readable comment, emitted as `COMMENT ON TABLE`
    /// (single quotes are escaped by doubling; it is a value, not an
    /// identifier).
    #[serde(default)]
    pub comment: Option<String>,
    /// When `true` (the safe default posture, FFS-001 D5) the generator
    /// unconditionally adds a `tenant_id text NOT NULL` column, enables and
    /// FORCEs row-level security, emits the four fixed tenant policies plus a
    /// tenant index, and grants CRUD to `authenticated`. The caller cannot
    /// supply, override, or disable any part of that block.
    #[serde(default)]
    pub tenant_scoped: bool,
    /// Escape hatch acknowledgement for `tenant_scoped: false` (FFS-001 §6):
    /// an unscoped table is refused unless this is explicitly `true`, and even
    /// then a warning is recorded in the plan and the ledger.
    #[serde(default)]
    pub acknowledge_unscoped: bool,
    /// Declared columns, in the order they will appear in `CREATE TABLE`.
    pub columns: Vec<ColumnSpec>,
    /// Caller-declared secondary indexes (validated: index name safe, every
    /// referenced column declared in this table).
    #[serde(default)]
    pub indexes: Vec<IndexSpec>,
    /// Advisory in v1: reflection exposes RLS-enabled tables automatically,
    /// so this flag is carried in the spec (and hashed) but not acted on by
    /// the generator. Reserved for a future exposure-control surface.
    #[serde(default = "default_true")]
    pub api_exposed: bool,
}

/// One declared column.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ColumnSpec {
    /// Column name (validated identifier; `tenant_id` is refused on
    /// tenant-scoped tables because the generator owns that column).
    pub name: String,
    /// Column type — a closed enum, never free text.
    #[serde(rename = "type")]
    pub column_type: ColumnType,
    /// Whether the column allows NULL. Defaults to `true` (nullable), the
    /// additive-safe choice for `ADD COLUMN`.
    #[serde(default = "default_true")]
    pub nullable: bool,
    /// Whether the column participates in the primary key.
    #[serde(default)]
    pub primary_key: bool,
    /// Optional default expression. Restricted to simple literals (quoted
    /// string without embedded quotes, integer/decimal, `true`/`false`) or
    /// the allowlisted functions `now()` / `gen_random_uuid()` — validated
    /// before generation, so it can be embedded verbatim.
    #[serde(default)]
    pub default: Option<String>,
}

/// The closed set of column types the provisioning API accepts (FFS-001
/// §4.2).
///
/// `#[non_exhaustive]` so adding a type later is not a breaking change for
/// downstream matches; serde uses the lowercase Postgres spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ColumnType {
    /// `text`
    Text,
    /// `integer`
    Integer,
    /// `bigint`
    Bigint,
    /// `numeric`
    Numeric,
    /// `boolean`
    Boolean,
    /// `date`
    Date,
    /// `timestamptz`
    Timestamptz,
    /// `uuid`
    Uuid,
    /// `jsonb`
    Jsonb,
}

impl ColumnType {
    /// The exact SQL type name emitted into generated DDL.
    #[must_use]
    pub fn sql_name(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Integer => "integer",
            Self::Bigint => "bigint",
            Self::Numeric => "numeric",
            Self::Boolean => "boolean",
            Self::Date => "date",
            Self::Timestamptz => "timestamptz",
            Self::Uuid => "uuid",
            Self::Jsonb => "jsonb",
        }
    }
}

/// One caller-declared secondary index.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IndexSpec {
    /// Index name (validated identifier).
    pub name: String,
    /// Columns the index covers, in order. Every entry must be a column
    /// declared on the same table (or `tenant_id` on a tenant-scoped table).
    pub columns: Vec<String>,
    /// Whether the index is `UNIQUE`.
    #[serde(default)]
    pub unique: bool,
}

fn default_true() -> bool {
    true
}
