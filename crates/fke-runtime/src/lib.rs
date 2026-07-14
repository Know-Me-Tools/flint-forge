//! Wasmtime host — WASM Component Model execution engine for Flint Kiln.
//!
//! # Architecture
//!
//! ```text
//! Data-plane (always):
//!   EdgeRuntime::new()       → Engine + ProxyPre cache
//!   EdgeRuntime::load_wasm() → Component::from_binary → ProxyPre + cache
//!   EdgeRuntime::handle()    → Cedar gate → ProxyPre::instantiate_async →
//!                              new_incoming_request + new_response_outparam →
//!                              call_handle → oneshot response
//!
//! Control-plane (compiler feature):
//!   AotCompiler::precompile(wasm) → .cwasm bytes (Cranelift AOT)
//! ```
//!
//! # Security
//!
//! - `Pep::check(caller, kiln:invoke)` fires before instantiation.
//!   `caller = None` skips Cedar (BGW / system-level invocation).
//! - Fuel limit prevents infinite loops.
//! - `#![forbid(unsafe_code)]` — safe `Component::from_binary` only.
#![forbid(unsafe_code)]

mod capability;
#[cfg(feature = "compiler")]
mod compiler;
mod helpers;
mod runtime;
mod types;

pub use capability::check_capabilities;
#[cfg(feature = "compiler")]
pub use compiler::AotCompiler;
pub use runtime::EdgeRuntime;
pub use types::{KilnHandleOutcome, KilnRequest, KilnResponse};
