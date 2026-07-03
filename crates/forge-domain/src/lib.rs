//! Forge core domain — pure cross-cutting types. No infrastructure dependencies.
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Tenant identifier (newtype over UUID-as-string for transport stability).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct TenantId(pub String);

/// Authenticated subject identifier (the JWT `sub`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct SubjectId(pub String);

/// JSON value alias used across ports.
pub type Json = serde_json::Value;

/// Top-level error surfaced across subsystem boundaries.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ForgeError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("not found")]
    NotFound,
    #[error("backend: {0}")]
    Backend(String),
    #[error("policy denied")]
    PolicyDenied,
}

/// Maximum length of a single Postgres identifier segment (`NAMEDATALEN - 1`).
const MAX_IDENTIFIER_LEN: usize = 63;

/// Reserved SQL keywords that must never appear unquoted as a bare identifier.
///
/// This is a deliberately small denylist covering the keywords most likely to
/// change query semantics if injected via a table/column name. It is a defence
/// in depth: the character-class check already rejects whitespace and
/// punctuation, so any surviving candidate is a lone word — the keywords here
/// are the dangerous lone words.
const RESERVED_KEYWORDS: &[&str] = &[
    "select", "insert", "update", "delete", "drop", "alter", "create", "grant", "revoke",
    "truncate", "union", "where", "from", "join", "table", "into",
];

/// Validate a SQL identifier (table or column name) before it is interpolated
/// into a query string.
///
/// This is the **single chokepoint** for every identifier that reaches SQL by
/// interpolation rather than parameter binding. Values (never identifiers) must
/// always be bound as `$1`, `$2`, …; identifiers cannot be parameterised in
/// Postgres, so they pass through here instead.
///
/// A name is safe when **every** dot-separated segment (to allow
/// `schema.table`) satisfies all of:
/// - non-empty and at most [`MAX_IDENTIFIER_LEN`] chars (Postgres truncates
///   beyond this, which would silently retarget the query);
/// - starts with an ASCII letter or underscore;
/// - contains only ASCII letters, digits, and underscores;
/// - is not a reserved keyword (case-insensitive, see [`RESERVED_KEYWORDS`]).
///
/// The empty string, a leading/trailing dot, or an empty segment (`a..b`) are
/// all rejected because they produce an empty segment.
#[must_use]
pub fn is_safe_identifier(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    name.split('.').all(is_safe_segment)
}

/// Validate one dot-free identifier segment. See [`is_safe_identifier`].
fn is_safe_segment(seg: &str) -> bool {
    if seg.is_empty() || seg.len() > MAX_IDENTIFIER_LEN {
        return false;
    }

    let mut chars = seg.chars();
    let first = chars.next().expect("segment is non-empty");
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return false;
    }

    !RESERVED_KEYWORDS
        .iter()
        .any(|kw| seg.eq_ignore_ascii_case(kw))
}

#[cfg(test)]
mod identifier_tests {
    use super::{is_safe_identifier, MAX_IDENTIFIER_LEN};

    #[test]
    fn accepts_valid_names() {
        assert!(is_safe_identifier("items"));
        assert!(is_safe_identifier("public"));
        assert!(is_safe_identifier("public.items"));
        assert!(is_safe_identifier("my_table_2"));
        assert!(is_safe_identifier("_private"));
        assert!(is_safe_identifier("schema.my_table"));
    }

    #[test]
    fn rejects_injection_attempts() {
        assert!(!is_safe_identifier("items; DROP TABLE users--"));
        assert!(!is_safe_identifier("items' OR '1'='1"));
        assert!(!is_safe_identifier("items--"));
        assert!(!is_safe_identifier("a b"));
        assert!(!is_safe_identifier("items\n"));
        assert!(!is_safe_identifier("col)"));
        assert!(!is_safe_identifier("1col")); // must not start with a digit
    }

    #[test]
    fn rejects_empty_and_dot_edge_cases() {
        assert!(!is_safe_identifier(""));
        assert!(!is_safe_identifier("."));
        assert!(!is_safe_identifier(".items"));
        assert!(!is_safe_identifier("items."));
        assert!(!is_safe_identifier("a..b"));
    }

    #[test]
    fn rejects_reserved_keywords_case_insensitive() {
        assert!(!is_safe_identifier("select"));
        assert!(!is_safe_identifier("DROP"));
        assert!(!is_safe_identifier("Delete"));
        assert!(!is_safe_identifier("public.select")); // any segment reserved => reject
                                                       // A reserved word as a substring of a longer name is fine.
        assert!(is_safe_identifier("selected"));
        assert!(is_safe_identifier("user_table"));
    }

    #[test]
    fn rejects_oversized_names() {
        let ok = "a".repeat(MAX_IDENTIFIER_LEN);
        assert!(is_safe_identifier(&ok));
        let too_long = "a".repeat(MAX_IDENTIFIER_LEN + 1);
        assert!(!is_safe_identifier(&too_long));
    }
}
