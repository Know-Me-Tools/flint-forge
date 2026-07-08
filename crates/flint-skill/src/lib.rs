//! Ergonomic Rust SDK for authoring Flint Kiln edge-function skills.
//!
//! `flint-skill` is the consumer-side helper crate that skill authors include
//! alongside their WIT-generated bindings. It provides:
//!
//! - **Typed errors** — a single [`SkillError`] covering every
//!   `flint:host@0.1.0` host interface, with machine-stable codes for
//!   metrics and Cedar-deny detection.
//! - **Helper types** — [`LlmOptions`], [`CompletionResult`], [`EmbeddingResult`],
//!   and [`DbRow`] for the JSON-encoded payloads the WIT surface exchanges.
//! - **Trait abstractions** — [`Database`], [`Llm`], [`Kv`], [`Identity`],
//!   [`Secrets`], and [`SecretHandle`] describe the host contract in ergonomic
//!   Rust. The skill author implements each trait as a one-line adapter over
//!   their `wit-bindgen`-generated `bindings::flint::host::*` module.
//!
//! The crate compiles on any target (including `wasm32-wasip2`) because it
//! contains **no WIT calls of its own**. All actual host calls live in the
//! skill author's component crate; this crate only provides the typed
//! scaffolding around them.
//!
//! See the [crate-level README](https://github.com/prometheus-ags/flint-forge/blob/main/crates/flint-skill/README.md)
//! for a complete `hello-world` skill example.
//!
//! # Stability
//!
//! All types in this crate track `flint:host@0.1.0`. Breaking changes in the
//! WIT world will bump this crate's minor version and be announced in
//! `docs/api/kiln-abi.md`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod db;
pub mod error;
pub mod identity;
pub mod kv;
pub mod llm;
pub mod secrets;
pub mod types;

pub use db::Database;
pub use error::{HostInterface, SkillError, SkillResult};
pub use identity::Identity;
pub use kv::Kv;
pub use llm::Llm;
pub use secrets::{SecretHandle, Secrets};
pub use types::{CompletionResult, DbRow, EmbeddingResult, LlmOptions};
