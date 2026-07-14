//! `flint:host/llm` — governed inference.
//!
//! The WIT `llm` interface exposes two functions:
//!
//! - `complete(prompt, opts) -> result<string, host-error>`
//! - `embed(input, model)   -> result<list<f32>, host-error>`
//!
//! All inference routes through `flint-gate` / the Universal Agent Router.
//! The component never holds a provider key; the host injects credentials
//! at the boundary via Flint Vault.

use crate::error::SkillResult;
use crate::types::{CompletionResult, EmbeddingResult, LlmOptions};
use std::future::Future;

/// Governed LLM access for Flint skills.
///
/// Implement this trait as a thin adapter over the WIT-generated
/// `bindings::flint::host::llm` module. See the crate README for a complete
/// example.
//
// Methods return `impl Future<Output = …> + Send` rather than `async fn` so
// that implementors can rely on `Send` futures without a future breaking
// change. See the clippy::async_fn_in_trait lint.
pub trait Llm {
    /// Generate a completion for `prompt` with the given [`LlmOptions`].
    ///
    /// `opts` is serialized to JSON via [`LlmOptions::to_json`] before being
    /// forwarded to the host, matching the WIT `opts: string` parameter.
    ///
    /// # Errors
    /// Implementations should map a WIT `host-error` from
    /// `flint:host/llm.complete` to [`crate::SkillError::Llm`] via
    /// [`crate::SkillError::from_host_error`] (e.g. `"PROVIDER_429"` or
    /// `"MODEL_UNKNOWN"`).
    fn complete<'a>(
        &'a self,
        prompt: &'a str,
        opts: &'a LlmOptions,
    ) -> impl Future<Output = SkillResult<CompletionResult>> + Send;

    /// Generate an embedding vector for `input`.
    ///
    /// If `model` is `None`, the host uses the publisher's Cedar-allowed
    /// default embedding model.
    ///
    /// # Errors
    /// Implementations should map a WIT `host-error` from
    /// `flint:host/llm.embed` to [`crate::SkillError::Llm`] via
    /// [`crate::SkillError::from_host_error`].
    fn embed<'a>(
        &'a self,
        input: &'a str,
        model: Option<&'a str>,
    ) -> impl Future<Output = SkillResult<EmbeddingResult>> + Send;
}
