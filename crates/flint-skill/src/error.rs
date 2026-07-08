//! Error types for Flint Kiln skill authors.
//!
//! Every host capability (`db`, `llm`, `secrets`, …) signals failure with a
//! WIT `host-error { code: string, message: string }`. [`SkillError`] is the
//! single concrete error type skill code maps those failures into, so that a
//! component's `incoming-handler` can produce a uniform 5xx response without
//! hand-rolling match arms per interface.
//!
//! `SkillError` is also the error type used by the helper wrappers in this
//! crate ([`crate::types`]). Skill authors who call through the WIT bindings
//! directly should convert with [`SkillError::from_host_error`].

use std::borrow::Cow;

/// Result alias used by every wrapper in this crate.
pub type SkillResult<T> = std::result::Result<T, SkillError>;

/// Machine-stable error categories for Flint skills.
///
/// Variants line up one-to-one with the five host interfaces in
/// `flint:host@0.1.0`. Each carries the `(code, message)` pair the host
/// returned, plus enough structure for skill code to switch on the interface
/// that failed without string matching.
#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    /// `flint:host/db` returned a `host-error`.
    #[error("db error: {code}: {message}")]
    Db {
        /// Machine-readable code from the host (e.g. `"SQL_PARSE"`, `"CEDAR_DENY"`).
        code: String,
        /// Human-readable detail from the host. Never contains JWT payloads
        /// or secret values per the WIT contract.
        message: String,
    },

    /// `flint:host/llm` returned a `host-error`.
    #[error("llm error: {code}: {message}")]
    Llm {
        /// Machine-readable code (e.g. `"PROVIDER_429"`, `"MODEL_UNKNOWN"`).
        code: String,
        /// Human-readable detail from the host.
        message: String,
    },

    /// `flint:host/secrets` returned a `host-error` on `get` or `reveal`.
    #[error("secrets error: {code}: {message}")]
    Secrets {
        /// Machine-readable code (e.g. `"CEDAR_DENY"`, `"NOT_FOUND"`).
        code: String,
        /// Human-readable detail from the host.
        message: String,
    },

    /// A byte payload from the host was not valid UTF-8. Should never happen
    /// for the JSON-encoded strings the WIT surface returns, but skills must
    /// never `unwrap()` host data.
    #[error("utf-8 decode of host payload failed: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    /// A host call returned a JSON string that did not deserialize into the
    /// expected Rust shape. Carries the raw payload so skill authors can log
    /// it for debugging without re-fetching from the host.
    #[error("json decode of host payload failed: {source}; payload: {payload}")]
    Json {
        /// Underlying serde error.
        #[source]
        source: serde_json::Error,
        /// Raw JSON the host returned, for diagnostics.
        payload: String,
    },

    /// A WIT resource handle (e.g. `secrets::secret`) was used after the host
    /// invalidated it. Components generally cannot observe this in v0.1.0
    /// because handles are bound to a single incoming-handler invocation,
    /// but it is provided for forward compatibility.
    #[error("host resource handle is stale or dropped")]
    StaleHandle,
}

impl SkillError {
    /// Build a [`SkillError`] from a `(code, message)` pair returned by any
    /// host interface, tagged by which interface produced it.
    ///
    /// Skill authors who call the WIT bindings directly use this in place of
    /// `?` to convert the generated `HostError` records into a single error
    /// type:
    ///
    /// ```no_run
    /// # use flint_skill::{SkillError, HostInterface};
    /// # let code = "SQL_PARSE".to_string();
    /// # let message = "bad placeholder".to_string();
    /// let err = SkillError::from_host_error(HostInterface::Db, code, message);
    /// ```
    #[must_use]
    pub fn from_host_error(iface: HostInterface, code: String, message: String) -> Self {
        match iface {
            HostInterface::Db => Self::Db { code, message },
            HostInterface::Llm => Self::Llm { code, message },
            HostInterface::Secrets => Self::Secrets { code, message },
            // kv, identity, and the wasi:http imports do not return host-error
            // records in flint:host@0.1.0. Treat the call as a programming
            // error and surface it as a stale-handle style invariant failure.
            HostInterface::Kv | HostInterface::Identity => Self::StaleHandle,
        }
    }

    /// Borrow the machine-readable code, if the variant carries one.
    ///
    /// Useful for metrics labels: `match err.code() { Some("CEDAR_DENY") => … }`.
    #[must_use]
    pub fn code(&self) -> Option<&str> {
        match self {
            Self::Db { code, .. } | Self::Llm { code, .. } | Self::Secrets { code, .. } => {
                Some(code.as_str())
            }
            Self::Utf8(_) | Self::Json { .. } | Self::StaleHandle => None,
        }
    }

    /// Borrow the human-readable message, if the variant carries one.
    ///
    /// Falls back to the variant's `Display` rendering for non-host variants
    /// (UTF-8 / JSON / stale-handle), so callers always have *something* to log.
    #[must_use]
    pub fn message(&self) -> Cow<'_, str> {
        match self {
            Self::Db { message, .. }
            | Self::Llm { message, .. }
            | Self::Secrets { message, .. } => Cow::Borrowed(message.as_str()),
            Self::Utf8(e) => Cow::Owned(e.to_string()),
            Self::Json { source, .. } => Cow::Owned(source.to_string()),
            Self::StaleHandle => Cow::Borrowed("host resource handle is stale or dropped"),
        }
    }

    /// Which host interface produced this error.
    #[must_use]
    pub fn interface(&self) -> HostInterface {
        match self {
            Self::Db { .. } => HostInterface::Db,
            Self::Llm { .. } => HostInterface::Llm,
            Self::Secrets { .. } => HostInterface::Secrets,
            // Generic decode / lifecycle errors are not attributable to one
            // interface; we report the one the WIT surface does not return
            // errors for so callers can distinguish "no host-error here" from
            // a real interface failure.
            Self::Utf8(_) | Self::Json { .. } | Self::StaleHandle => HostInterface::Kv,
        }
    }
}

/// Tags which `flint:host` interface a value came from.
///
/// Used as the first argument to [`SkillError::from_host_error`] so that a
/// single conversion entry point covers every WIT error record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostInterface {
    /// `flint:host/db` — governed SQL.
    Db,
    /// `flint:host/llm` — governed inference.
    Llm,
    /// `flint:host/kv` — ephemeral per-invocation store (never returns errors).
    Kv,
    /// `flint:host/identity` — verified origin JWT claims (never returns errors).
    Identity,
    /// `flint:host/secrets` — Cedar-gated secret access.
    Secrets,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_host_error_tags_db() {
        let e = SkillError::from_host_error(
            HostInterface::Db,
            "SQL_PARSE".into(),
            "bad placeholder".into(),
        );
        assert_eq!(e.code(), Some("SQL_PARSE"));
        assert_eq!(e.message(), "bad placeholder");
        assert_eq!(e.interface(), HostInterface::Db);
        assert_eq!(e.to_string(), "db error: SQL_PARSE: bad placeholder");
    }

    #[test]
    fn from_host_error_tags_llm() {
        let e = SkillError::from_host_error(
            HostInterface::Llm,
            "PROVIDER_429".into(),
            "rate limited".into(),
        );
        assert_eq!(e.code(), Some("PROVIDER_429"));
        assert_eq!(e.interface(), HostInterface::Llm);
    }

    #[test]
    fn from_host_error_tags_secrets() {
        let e = SkillError::from_host_error(
            HostInterface::Secrets,
            "CEDAR_DENY".into(),
            "no grant".into(),
        );
        assert_eq!(e.code(), Some("CEDAR_DENY"));
        assert_eq!(e.interface(), HostInterface::Secrets);
    }

    #[test]
    fn kv_and_identity_map_to_invariant() {
        // These interfaces do not return host-error in v0.1.0; passing them
        // is a programming error and must surface as a non-host variant.
        let e = SkillError::from_host_error(HostInterface::Kv, "x".into(), "y".into());
        assert!(matches!(e, SkillError::StaleHandle));
        assert_eq!(e.code(), None);
    }

    #[test]
    fn json_error_carries_payload() {
        let source = serde_json::from_str::<serde_json::Value>("{bad}").unwrap_err();
        let e = SkillError::Json {
            source,
            payload: "{bad}".into(),
        };
        // The payload is preserved so skill code can log it without a re-fetch.
        let SkillError::Json { payload, .. } = e else {
            panic!("wrong variant");
        };
        assert_eq!(payload, "{bad}");
    }
}
