//! The reflection pipeline passes `ReflectionEngine::reflect` runs, in order,
//! over a freshly-queried `DatabaseModel`:
//!
//! 1. `normalization` — canonicalize schema names/types, deduplicate columns.
//! 2. `validation` — reject structurally invalid models before they reach
//!    the compilers (empty tables, dangling FKs, dangerous identifiers).
//! 3. `permission_analysis` — warn on RLS-less tables exposed to `anon`.
//!
//! `endpoint_generation` is a separate, on-demand pass consumed directly by
//! `RestCompiler::compile_with_gates` rather than by `reflect()` itself.

/// On-demand pass: derive the ordered list of HTTP [`endpoint_generation::Endpoint`]s
/// from a validated `DatabaseModel`, consumed by `RestCompiler::compile_with_gates`.
pub mod endpoint_generation;
/// Pass 1: canonicalize schema names/Postgres type aliases and deduplicate columns.
pub mod normalization;
/// Pass 3: warn (does not yet block) on tables with RLS disabled.
pub mod permission_analysis;
/// Pass 2: reject structurally invalid models (empty tables, dangling FKs,
/// dangerous identifiers) before they reach the compilers.
pub mod validation;
