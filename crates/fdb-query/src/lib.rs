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

pub mod ident;
pub mod operator;
pub mod param;

pub use ident::{ColumnRef, IdentError, parse_column_ref, validate_identifier};
pub use operator::{Operator, Quantifier, RenderError, render_condition};
pub use param::{QueryParam, pg_text_array};
