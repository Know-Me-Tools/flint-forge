//! `fdb-query` — a pure, I/O-free PostgREST-compatible request→SQL translator.
//!
//! This crate parses PostgREST request grammar (horizontal filtering, logical
//! trees, `select`/`order`/pagination, writes) into a typed query plan and renders
//! `(sql, params)` where every user value is a bound parameter (`$n`) and every
//! identifier is validated. It has **no database dependency** and **no async** —
//! the executor adapter (`fdb-postgres`) binds the params and runs the SQL under
//! the RLS context. Purity is deliberate: it makes the full operator/edge-case
//! surface unit-testable without a live Postgres.
//!
//! # Layering
//! Layer 0/1 — consumed by both `fdb-reflection` (REST router) and `fdb-postgres`
//! (`PgRest`). Depends only on `serde`/`thiserror`/`forge-domain`. Never sees
//! `RlsContext`; RLS is enforced by the executor's `SET LOCAL` GUCs.
#![forbid(unsafe_code)]

pub mod cast;
pub mod clause;
pub mod embed;
pub mod filter;
pub mod fts;
pub mod ident;
pub mod mutation;
pub mod operator;
pub mod param;
pub mod plan;

pub use cast::CastHints;
pub use clause::{CountStrategy, Limits, Order, Select};
pub use embed::{
    parse_embed_select, render_inner_guards, render_projection, resolve_embeds,
    route_embedded_param, Cardinality, EmbedError, EmbedRequest, EmbedSchema, EmbedSelect, FkEdge,
    JoinKind, ResolvedEmbed, ScalarCol, TableDesc,
};
pub use filter::{FilterError, FilterTree};
pub use fts::{render_fts, FtsConfig, FtsKind};
pub use ident::{parse_column_ref, validate_identifier, ColumnRef, IdentError};
pub use mutation::{
    parse_write_prefer, DeletePlan, InsertOptions, InsertPlan, Resolution, ReturnKind, UpdatePlan,
};
pub use operator::{render_condition, Operator, Quantifier, RenderError};
pub use param::{pg_text_array, QueryParam};
pub use plan::{parse_select_request, ParseError, SelectPlan, RESERVED_PARAMS};
