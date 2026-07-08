//! Helper types for the LLM and DB host interfaces.
//!
//! The WIT `flint:host@0.1.0` surface carries all structured payloads as
//! JSON-encoded strings (WIT has no native `json` type). These types give
//! skill authors a single serde-validated shape to encode and decode those
//! strings into, instead of hand-rolling `serde_json::Value` access in every
//! component.

use crate::error::{SkillError, SkillResult};
use serde::{Deserialize, Serialize};

/// Builder-style options passed to [`crate::Llm::complete`]-style call sites.
///
/// Mirrors the JSON object the host expects as the `opts` parameter to
/// `flint:host/llm.complete`. Every field is optional; the host applies its
/// own defaults for any field the skill omits when the struct is serialized
/// with `skip_serializing_none` (see [`LlmOptions::to_json`]).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LlmOptions {
    /// Provider model identifier, e.g. `"gpt-4o-mini"` or `"claude-3-5-sonnet"`.
    /// If `None`, the host uses the publisher's Cedar-allowed default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Sampling temperature in `[0.0, 2.0]`. `None` = provider default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Maximum output tokens to generate. `None` = provider default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Provider-specific extras forwarded verbatim. Use sparingly — fields
    /// here are not portable across providers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Map<String, serde_json::Value>>,
}

impl LlmOptions {
    /// Build an empty options set. Equivalent to `Default::default()` but
    /// reads more naturally at the top of a skill function.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the model identifier.
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the sampling temperature.
    #[must_use]
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set the maximum output tokens.
    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Serialize to the JSON string the `flint:host/llm.complete` WIT import
    /// expects as its `opts` parameter.
    ///
    /// Errors only if the struct contains a non-serializable value inside
    /// `extra` (e.g. a serde_json::Number that is not finite). That is a
    /// programming error in skill code; surface it via [`SkillError::Json`].
    ///
    /// [`SkillError::Json`]: crate::SkillError::Json
    pub fn to_json(&self) -> SkillResult<String> {
        serde_json::to_string(self).map_err(|source| SkillError::Json {
            source,
            payload: String::from("<LlmOptions>"),
        })
    }
}

/// Decoded result of a single `flint:host/llm.complete` call.
///
/// The host returns the completion text directly through the WIT import;
/// this struct is the ergonomic wrapper skill code reads when it goes through
/// [`crate::Llm::complete`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionResult {
    /// The completion text. Never `None` on a successful call.
    pub text: String,
}

/// Decoded result of a single `flint:host/llm.embed` call.
///
/// The WIT import returns `list<f32>` directly. This wrapper exists so that
/// skill code has a named type to thread through pipelines and so that future
/// host-side metadata (model id, dimensionality) can land without breaking
/// the call signature.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingResult {
    /// The embedding vector, in the model's native dimensionality.
    pub vector: Vec<f32>,
}

/// Decoded row from `flint:host/db.query`.
///
/// The host returns each row as a JSON-encoded object whose keys are the
/// selected column names. Skills usually want to keep rows as
/// `serde_json::Value` (the keys are schema-dependent), so this is just a
/// transparent newtype around `serde_json::Value` with a few accessors.
///
/// For typed decoding, use [`DbRow::into_typed`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DbRow(pub serde_json::Value);

impl DbRow {
    /// Construct a row from a raw JSON string returned by the host.
    ///
    /// Used by [`Database::query`] implementations after a successful WIT
    /// `flint:host/db.query` call to decode each JSON-encoded row.
    ///
    /// [`Database::query`]: crate::Database::query
    pub fn from_json_str(s: &str) -> SkillResult<Self> {
        let value: serde_json::Value =
            serde_json::from_str(s).map_err(|source| SkillError::Json {
                source,
                payload: s.to_string(),
            })?;
        Ok(Self(value))
    }

    /// Borrow the row as a JSON object, or `None` if the host returned a
    /// non-object JSON value (which would indicate a host bug).
    #[must_use]
    pub fn as_object(&self) -> Option<&serde_json::Map<String, serde_json::Value>> {
        self.0.as_object()
    }

    /// Borrow a column by name.
    #[must_use]
    pub fn get(&self, column: &str) -> Option<&serde_json::Value> {
        self.0.get(column)
    }

    /// Decode the row into a domain type. Skills typically define a `FromRow`
    /// impl per query and call `row.into_typed::<User>()`.
    ///
    /// Errors if any field is missing or has the wrong type. The error
    /// carries the offending row payload for diagnostics.
    pub fn into_typed<T: for<'de> Deserialize<'de>>(self) -> SkillResult<T> {
        serde_json::from_value(self.0.clone()).map_err(|source| SkillError::Json {
            source,
            payload: self.0.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_options_round_trip_skips_none() {
        let opts = LlmOptions::new()
            .with_model("gpt-4o-mini")
            .with_temperature(0.2);
        let json = opts.to_json().unwrap();
        // skip_serializing_if = None should drop max_tokens and extra.
        assert!(json.contains("\"model\":\"gpt-4o-mini\""));
        assert!(json.contains("\"temperature\":0.2"));
        assert!(!json.contains("max_tokens"));
        assert!(!json.contains("extra"));
    }

    #[test]
    fn llm_options_empty_is_empty_object() {
        let json = LlmOptions::new().to_json().unwrap();
        assert_eq!(json, "{}");
    }

    #[test]
    fn db_row_from_json_str_decodes() {
        let row = DbRow::from_json_str(r#"{"id": 7, "name": "ada"}"#).unwrap();
        assert_eq!(row.get("id"), Some(&serde_json::json!(7)));
        assert_eq!(row.get("name").and_then(|v| v.as_str()), Some("ada"));
    }

    #[test]
    fn db_row_into_typed_succeeds() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct User {
            id: u32,
            name: String,
        }
        let row = DbRow::from_json_str(r#"{"id":7,"name":"ada"}"#).unwrap();
        let user: User = row.into_typed().unwrap();
        assert_eq!(
            user,
            User {
                id: 7,
                name: "ada".into(),
            }
        );
    }

    #[test]
    fn db_row_into_typed_preserves_payload_on_error() {
        #[derive(Deserialize, Debug)]
        #[allow(dead_code)]
        struct User {
            id: u32,
        }
        let row = DbRow(serde_json::json!({"id": "not-a-number"}));
        let err = row.into_typed::<User>().unwrap_err();
        match err {
            SkillError::Json { payload, .. } => assert!(payload.contains("not-a-number")),
            other => panic!("expected Json, got {other:?}"),
        }
    }
}
