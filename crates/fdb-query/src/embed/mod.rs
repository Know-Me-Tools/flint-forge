//! Resource embedding — PostgREST `select=*,child(*)` → correlated JSON subselects.
//!
//! Embedding is expressed as a **caller-supplied** schema descriptor
//! ([`EmbedSchema`]) plus a recursive request tree parsed from the `select=`
//! grammar. Each embedded relation renders as a *correlated scalar subquery* in
//! the parent SELECT list — never a top-level JOIN — because a PostgREST embed
//! produces exactly one JSON value per parent row (an array for to-many, an
//! object for to-one) carrying its own independent filter/order/limit. A LEFT
//! JOIN would multiply parent rows and could not carry a per-embed `LIMIT`.
//!
//! * to-many → `COALESCE((SELECT json_agg(...) FROM child c WHERE corr AND flt), '[]'::json)`
//! * to-one  → `(SELECT json_build_object(...) FROM child c WHERE corr AND flt LIMIT 1)`
//! * `!inner` / top-level filter-by-embed → `AND EXISTS (SELECT 1 FROM child ...)`
//! * `...child(col)` (spread, to-one only) → flattened scalar subselects
//!
//! Every identifier reaching SQL is validated ([`crate::ident`]) and every user
//! value is a bound `$n` ([`crate::param::QueryParam`]); the shared `next_index`
//! counter is threaded through the parent WHERE, every embed subselect, and
//! spread items so the emitted params are globally unique and in `$1..$n` order.
//!
//! This crate has **no** knowledge of `fdb-reflection`; the reflection layer maps
//! its `DatabaseModel` FK metadata into [`FkEdge`]s and passes an [`EmbedSchema`].
//!
//! ## Layout
//! * [`schema`] — descriptor + parsed/resolved tree types + [`EmbedError`]
//! * [`parse`] — schema-free `select=` parsing and embedded-param routing
//! * [`render`] — schema-aware resolution and SQL rendering

mod parse;
mod render;
mod schema;

#[cfg(test)]
mod tests;

pub use parse::{parse_embed_select, route_embedded_param};
pub use render::{render_inner_guards, render_projection, resolve_embeds};
pub use schema::{
    Cardinality, EmbedError, EmbedRequest, EmbedSchema, EmbedSelect, FkEdge, JoinKind,
    ResolvedEmbed, ScalarCol, TableDesc,
};
