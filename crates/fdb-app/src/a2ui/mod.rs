//! A2UI component registry use-cases for the Flint Quarry application layer.
//!
//! This module provides types and use-case logic for working with the
//! `flint_a2ui` component registry. It does NOT import adapter crates —
//! all database access goes through the port traits in `fdb-ports`.

pub mod types;

pub use types::{DesignToken, DesignTokenMap, Renderers, ResolvedComponent};
