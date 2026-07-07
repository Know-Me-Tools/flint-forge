//! Error types for Flint Ember LLM operations.

use std::time::Duration;

/// Result alias used throughout `flint_llm`.
pub type Result<T> = std::result::Result<T, LlmError>;

/// Failure modes for flint-gate/UAR calls and credential resolution.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("invalid response from gateway: {0}")]
    BadResponse(String),

    #[error("gateway error {code}: {message}")]
    Gateway { code: u16, message: String },

    #[error("request timed out after {0:?}")]
    Timeout(Duration),

    #[error("request was interrupted")]
    Interrupted,

    #[error("credential resolution failed: {0}")]
    Credential(String),

    #[error("configuration error: {0}")]
    Config(String),
}

impl LlmError {
    /// Construct from an HTTP response that carried an error body.
    pub fn from_response(code: u16, body: String) -> Self {
        Self::Gateway {
            code,
            message: body,
        }
    }
}
