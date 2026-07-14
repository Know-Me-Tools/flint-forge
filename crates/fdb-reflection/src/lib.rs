//! `fdb-reflection` — Quarry schema reflection adapter.
//!
//! Queries `flint_meta.*` (installed by Phase 1 `ext-flint-meta`) and produces
//! a `DatabaseModel` IR that drives REST routing, OpenAPI generation, and
//! ArcSwap hot-reload.
//!
//! # Hexagonal rule
//! This crate is an **adapter** (Layer 1.5). It MUST NOT import `fdb-gateway`
//! (the interface layer). Composition happens only in `fdb-gateway`.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Compiled, hot-swappable routing/schema state ([`CompiledState`]) and the
/// A2UI catalog types it carries.
pub mod compiled;
/// The REST/GraphQL/OpenAPI/MCP/A2UI compilers that turn a [`model::DatabaseModel`]
/// into servable artifacts (Axum router, OpenAPI doc, GraphQL schema, MCP tool
/// list, A2UI assembler).
pub mod compilers;
/// [`ReflectionEngine`] — queries `flint_meta.*` and assembles a [`model::DatabaseModel`].
pub mod engine;
/// [`ReflectionError`] — the error type shared by the engine, passes, and compilers.
pub mod error;
/// The `DatabaseModel` IR and its constituent types (tables, columns, functions, views).
pub mod model;
/// The pipeline passes run over a freshly-reflected `DatabaseModel` (normalize,
/// validate, analyze permissions) plus the on-demand endpoint-generation pass.
pub mod passes;
/// [`StateManager`] — owns the `ArcSwap<CompiledState>` and the background
/// `PgListener` loop that triggers recompilation on DDL change.
pub mod state_manager;

pub use compiled::{A2uiCatalog, A2uiCatalogEntry, CompiledState};
pub use engine::ReflectionEngine;
pub use error::ReflectionError;
pub use model::{
    ArgMeta, Column, DatabaseModel, EncryptedDek, FnMeta, ForeignKey, Table, ViewMeta,
};
pub use state_manager::{MutationGates, StateManager};
