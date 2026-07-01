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

pub mod compiled;
pub mod compilers;
pub mod engine;
pub mod error;
pub mod model;
pub mod passes;
pub mod state_manager;

pub use compiled::{A2uiCatalog, A2uiCatalogEntry, CompiledState};
pub use engine::ReflectionEngine;
pub use error::ReflectionError;
pub use model::{
    ArgMeta, Column, DatabaseModel, EncryptedDek, FnMeta, ForeignKey, Table, ViewMeta,
};
pub use state_manager::{MutationGates, StateManager};
