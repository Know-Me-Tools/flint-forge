//! Bind-parameter model.
//!
//! Every user-supplied value in a rendered query is a bound parameter (`$n`),
//! never interpolated. `QueryParam` is deliberately backend-agnostic: the
//! executor adapter (`fdb-postgres`) maps each variant onto a concrete
//! `tokio_postgres`/`sqlx` bind. Keeping this crate free of any DB driver is
//! what makes the whole translator pure and unit-testable without a database.

/// A single bind value, in `$n` order, produced by rendering a query plan.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum QueryParam {
    /// A text value bound as `text` (the common case; Postgres coerces at use site).
    Text(String),
    /// A Postgres array literal (e.g. `{a,b,c}`) bound as `text[]`-castable text.
    /// Produced by the `in`/`any`/`all` paths.
    TextArray(Vec<String>),
    /// A JSON value bound as `jsonb`-castable text. Used by containment operators
    /// (`cs`/`cd`) and JSON-path comparisons.
    Json(String),
    /// A SQL `NULL` placeholder. Rare — most nullability is expressed via the
    /// `is` operator's inlined literal, but bulk-insert `missing=default` uses it.
    Null,
}

impl QueryParam {
    /// The value a text-typed bind carries, if this is a [`QueryParam::Text`].
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s),
            _ => None,
        }
    }
}

/// Encode a list of strings as a Postgres array literal, e.g. `{"a","b","c"}`.
///
/// Each element is double-quoted and internal backslashes/quotes escaped, so the
/// literal is safe to bind as a single `text[]`-castable parameter. Empty input
/// yields `{}` (an empty array), which is the correct Postgres empty-array literal
/// and makes `= ANY($n)` evaluate to false for every row (PostgREST `in.()` semantics).
#[must_use]
pub fn pg_text_array(items: &[String]) -> String {
    let escaped: Vec<String> = items
        .iter()
        .map(|s| {
            let inner = s.replace('\\', "\\\\").replace('"', "\\\"");
            format!("\"{inner}\"")
        })
        .collect();
    format!("{{{}}}", escaped.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_array_quotes_and_escapes() {
        assert_eq!(pg_text_array(&["a".into(), "b".into()]), r#"{"a","b"}"#);
        assert_eq!(
            pg_text_array(&[r#"a"b"#.into()]),
            r#"{"a\"b"}"#,
            "double quotes are escaped"
        );
        assert_eq!(
            pg_text_array(&[r"a\b".into()]),
            r#"{"a\\b"}"#,
            "backslashes are escaped"
        );
    }

    #[test]
    fn text_array_empty_is_empty_literal() {
        assert_eq!(pg_text_array(&[]), "{}");
    }

    #[test]
    fn as_text_only_for_text_variant() {
        assert_eq!(QueryParam::Text("x".into()).as_text(), Some("x"));
        assert_eq!(QueryParam::Null.as_text(), None);
        assert_eq!(QueryParam::TextArray(vec![]).as_text(), None);
    }
}
