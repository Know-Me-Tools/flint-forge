//! Embed schema descriptor, parsed request tree, resolved tree, and errors.
//!
//! These are the data types shared across the `parse` and `render` submodules.
//! [`EmbedSchema`] is caller-supplied and serde-free; `fdb-reflection` maps its
//! `DatabaseModel` FK metadata into [`FkEdge`]s.

use std::collections::BTreeMap;

use crate::clause::{Order, OrderError};
use crate::filter::{FilterError, FilterTree};
use crate::ident::IdentError;


/// Caller-supplied, serde-free schema descriptor. `fdb-reflection` maps its
/// `DatabaseModel` → `EmbedSchema`; `fdb-query` never depends on `fdb-reflection`.
#[derive(Debug, Clone, Default)]
pub struct EmbedSchema {
    tables: BTreeMap<String, TableDesc>,
}

impl EmbedSchema {
    /// An empty schema (no embeddable relations).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a table descriptor (builder style).
    #[must_use]
    pub fn with_table(mut self, name: impl Into<String>, desc: TableDesc) -> Self {
        self.tables.insert(name.into(), desc);
        self
    }

    /// Look up a table by name.
    #[must_use]
    pub fn table(&self, name: &str) -> Option<&TableDesc> {
        self.tables.get(name)
    }
}

/// One table's embeddable surface: its columns (for validation) and FK edges.
#[derive(Debug, Clone, Default)]
pub struct TableDesc {
    /// Column names, used to validate embed projections and `*` expansion.
    pub columns: Vec<String>,
    /// FK edges usable for a join, in either direction.
    pub fks: Vec<FkEdge>,
}

impl TableDesc {
    /// An empty table descriptor.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a column (builder style).
    #[must_use]
    pub fn with_column(mut self, c: impl Into<String>) -> Self {
        self.columns.push(c.into());
        self
    }

    /// Add an FK edge (builder style).
    #[must_use]
    pub fn with_fk(mut self, e: FkEdge) -> Self {
        self.fks.push(e);
        self
    }
}

/// A directed FK relationship usable for a join:
/// `from_table.from_col -> to_table.to_col`, named `fk_name` for `!fk`
/// disambiguation. `cardinality` describes the traversal *parent → target*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FkEdge {
    /// Constraint name for `!fk` disambiguation.
    pub fk_name: String,
    /// Table holding the FK column.
    pub from_table: String,
    /// FK column on `from_table`.
    pub from_col: String,
    /// Referenced table.
    pub to_table: String,
    /// Referenced column on `to_table`.
    pub to_col: String,
    /// Whether embedding the target yields many rows or one.
    pub cardinality: Cardinality,
}

/// Cardinality of a parent→target traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Cardinality {
    /// Child references parent (parent embeds many children): to-many.
    ToMany,
    /// This row references one parent (embeds one): to-one.
    ToOne,
}

/// Requested join semantics for an embed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum JoinKind {
    /// Default `LEFT`: parent row kept even with no children (embed is `[]`/null).
    Left,
    /// `!inner`: parent row dropped unless the embed matches ≥1 child (adds `EXISTS`).
    Inner,
}

// ---------------------------------------------------------------------------
// Parsed (schema-free) request tree
// ---------------------------------------------------------------------------

/// A parsed, not-yet-resolved embedding request node (schema-free).
#[derive(Debug, Clone, PartialEq)]
pub struct EmbedRequest {
    /// Optional output alias (`alias:target(...)`).
    pub alias: Option<String>,
    /// The target relation name.
    pub target: String,
    /// Optional `!fk_name` disambiguation hint.
    pub fk_hint: Option<String>,
    /// Join semantics (`!inner` / `!left`).
    pub join: JoinKind,
    /// Whether this is a `...spread` embed.
    pub spread: bool,
    /// The embed's own projection (scalar columns + nested embeds).
    pub select: EmbedSelect,
    /// Routed embedded filters (`?target.col=eq.x`).
    pub filter: FilterTree,
    /// Routed embedded order (`order=target.col.desc`).
    pub order: Order,
    /// Routed embedded pagination (`target.limit=`/`target.offset=`).
    pub limit: Option<u64>,
    /// Routed embedded offset.
    pub offset: Option<u64>,
}

/// The projection inside a relation: plain scalar columns/renames plus nested
/// embeds. Scalar columns are kept as `(alias, column-ref)` pairs so a
/// `json_build_object` can name each key; `columns.is_empty()` means `*`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EmbedSelect {
    /// Scalar projection columns: `(output_key, raw_column_ref)`.
    pub columns: Vec<ScalarCol>,
    /// Nested embedded resources.
    pub embeds: Vec<EmbedRequest>,
}

/// One scalar projection column inside an embed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScalarCol {
    /// Output JSON key (alias if given, else the base column name).
    pub key: String,
    /// Raw column reference (validated at render time).
    pub col_ref: String,
    /// True when this token was a bare `*`.
    pub star: bool,
}

/// A resolved embed bound to a concrete [`FkEdge`] after consulting the schema.
#[derive(Debug, Clone)]
pub struct ResolvedEmbed {
    /// The originating (schema-free) request.
    pub request: EmbedRequest,
    /// The FK edge chosen for correlation.
    pub edge: FkEdge,
    /// SQL alias of the parent relation at this level.
    pub parent_alias: String,
    /// SQL alias assigned to the child relation.
    pub child_alias: String,
    /// Fully expanded scalar columns (`*` resolved to concrete names).
    pub columns: Vec<ScalarCol>,
    /// Resolved nested embeds.
    pub children: Vec<ResolvedEmbed>,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors from embed parsing / resolution / rendering. Each maps to HTTP 400.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum EmbedError {
    /// The requested relation is not present in the schema.
    #[error("unknown relation: {0}")]
    UnknownRelation(String),
    /// No FK path links the two relations.
    #[error("no FK path from `{from}` to `{to}`")]
    NoFkPath {
        /// Parent relation.
        from: String,
        /// Target relation.
        to: String,
    },
    /// Multiple FK paths exist and no `!fk` hint disambiguated them.
    #[error("ambiguous FK from `{from}` to `{to}`: candidates {candidates:?}")]
    AmbiguousFk {
        /// Parent relation.
        from: String,
        /// Target relation.
        to: String,
        /// Candidate FK constraint names.
        candidates: Vec<String>,
    },
    /// A `!fk` hint named a constraint that does not link the relations.
    #[error("unknown FK name: {0}")]
    UnknownFkName(String),
    /// A spread (`...`) embed was applied to a to-many edge.
    #[error("spread embed requires a to-one relationship: {0}")]
    SpreadRequiresToOne(String),
    /// An embedded projection referenced a column not on the child table.
    #[error("unknown column `{column}` on table `{table}`")]
    UnknownColumn {
        /// Child table name.
        table: String,
        /// Offending column.
        column: String,
    },
    /// The `select=` embed grammar was malformed (unbalanced parens, etc.).
    #[error("malformed embed: {0}")]
    MalformedEmbed(String),
    /// Identifier validation failed.
    #[error(transparent)]
    Ident(#[from] IdentError),
    /// A routed embedded filter failed to build/render.
    #[error(transparent)]
    Filter(#[from] FilterError),
    /// A routed embedded order failed to parse.
    #[error(transparent)]
    Order(#[from] OrderError),
}

/// The effective output name of an embed (alias if present, else target).
pub(super) fn embed_output_name(req: &EmbedRequest) -> &str {
    req.alias.as_deref().unwrap_or(&req.target)
}
