//! A2UI surface assembler.
//!
//! Turns an incoming event context into an A2UI v0.9.1 message sequence by
//! applying application-specific assembly rules, then falling back to the
//! default table → component binding in `flint_a2ui.bindings`.
#![forbid(unsafe_code)]

mod assembler;
mod error;
mod helpers;
mod rows;
mod types;

#[cfg(test)]
mod tests;

pub use assembler::A2uiAssembler;
pub use error::{A2uiPublisher, AssemblerError};
pub use types::{A2uiMessage, A2uiSurface, AssemblyContext};
