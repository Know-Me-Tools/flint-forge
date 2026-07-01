//! schema — DDL bootstrap is entirely handled by `sql/flint_meta.sql`.
//!
//! The `flint_meta` schema and all cache tables are created by the SQL file
//! referenced in `lib.rs` via `extension_sql_file!`. This module is reserved
//! for future Rust-side schema helpers when the reflection engine (Phase 2)
//! requires them.
