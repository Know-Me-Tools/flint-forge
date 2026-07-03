//! Identifier and column-reference validation — the security-critical layer.
//!
//! Everything user-controlled that reaches SQL *as text* (not as a bind) passes
//! through here. Plain identifiers are validated with
//! [`forge_domain::is_safe_identifier`] (segmented alphanumeric/underscore). A
//! PostgREST column reference extends that with JSON paths (`col->key`,
//! `col->>key`, `col->2`) and casts (`col::type`), each part re-validated before
//! it is emitted. No user text is ever interpolated without passing a validator.

use forge_domain::is_safe_identifier;

/// A validated, render-safe column reference.
///
/// Produced by [`parse_column_ref`]. `to_sql()` emits a fragment that is safe to
/// splice into a query (all constituent identifiers validated; JSON keys are
/// single-quoted string literals with quotes escaped).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnRef {
    /// The base column identifier (already validated).
    base: String,
    /// JSON path steps applied after the base column.
    json_path: Vec<JsonStep>,
    /// Optional Postgres cast target type (validated identifier), e.g. `int`.
    cast: Option<String>,
}

/// One step in a JSON access path.
#[derive(Debug, Clone, PartialEq, Eq)]
enum JsonStep {
    /// `-> 'key'` (object field, JSON result) or `-> n` (array index).
    Field { key: String, as_text: bool },
}

impl ColumnRef {
    /// Render the column reference to a SQL fragment.
    ///
    /// Base and cast are bare identifiers (validated). JSON keys are emitted as
    /// quoted string literals (`'key'`) with embedded single quotes doubled, so a
    /// malicious key cannot break out of the literal.
    #[must_use]
    pub fn to_sql(&self) -> String {
        let mut sql = self.base.clone();
        for step in &self.json_path {
            let JsonStep::Field { key, as_text } = step;
            let arrow = if *as_text { "->>" } else { "->" };
            if let Ok(idx) = key.parse::<i64>() {
                // Array index: numeric, no quoting needed.
                sql = format!("{sql} {arrow} {idx}");
            } else {
                let escaped = key.replace('\'', "''");
                sql = format!("{sql} {arrow} '{escaped}'");
            }
        }
        if let Some(cast) = &self.cast {
            sql = format!("({sql})::{cast}");
        }
        sql
    }

    /// The base column name (validated), without JSON path or cast.
    #[must_use]
    pub fn base(&self) -> &str {
        &self.base
    }
}

/// Validate a bare identifier (schema, table, column, relation, alias, cast type).
///
/// # Errors
/// Returns [`IdentError::Unsafe`] when the identifier fails the segmented
/// alphanumeric/underscore check.
pub fn validate_identifier(name: &str) -> Result<&str, IdentError> {
    if is_safe_identifier(name) {
        Ok(name)
    } else {
        Err(IdentError::Unsafe(name.to_owned()))
    }
}

/// Parse a PostgREST column reference: `base[ -> key | ->> key ]*[ :: type ]`.
///
/// Examples: `age`, `data->>name`, `payload->addr->>city`, `count::int`,
/// `meta->2::text`.
///
/// # Errors
/// Returns [`IdentError`] when the base column, a JSON key that looks like an
/// identifier, or the cast type fails validation. (Numeric JSON indices and
/// arbitrary string keys are allowed — string keys are emitted as escaped
/// literals, never as identifiers.)
pub fn parse_column_ref(input: &str) -> Result<ColumnRef, IdentError> {
    if input.is_empty() {
        return Err(IdentError::Empty);
    }

    // Split off an optional trailing `::type` cast first.
    let (head, cast) = match input.split_once("::") {
        Some((h, c)) => {
            let c = validate_identifier(c).map_err(|_| IdentError::UnsafeCast(c.to_owned()))?;
            (h, Some(c.to_owned()))
        }
        None => (input, None),
    };

    // Split the JSON path. The first token is the base column; subsequent tokens
    // follow `->` (json) or `->>` (text). We tokenize on the arrows explicitly so
    // that `->>` is not mis-read as `->` `>`.
    let (base_raw, steps) = split_json_path(head);
    let base = validate_identifier(base_raw)
        .map_err(|_| IdentError::Unsafe(base_raw.to_owned()))?
        .to_owned();

    Ok(ColumnRef {
        base,
        json_path: steps,
        cast,
    })
}

/// Tokenize `col->a->>b` into the base column and its ordered JSON steps.
fn split_json_path(head: &str) -> (&str, Vec<JsonStep>) {
    // Find the first arrow; everything before it is the base column.
    let Some(first_arrow) = head.find("->") else {
        return (head, vec![]);
    };
    let base = &head[..first_arrow];
    let mut rest = &head[first_arrow..];
    let mut steps = Vec::new();

    while !rest.is_empty() {
        // `->>` (text) must be checked before `->` (json).
        let (as_text, after_arrow) = if let Some(a) = rest.strip_prefix("->>") {
            (true, a)
        } else if let Some(a) = rest.strip_prefix("->") {
            (false, a)
        } else {
            // Not an arrow boundary — should not happen given we split on `->`.
            break;
        };
        // The key runs until the next arrow.
        let key_end = after_arrow.find("->").unwrap_or(after_arrow.len());
        let key = &after_arrow[..key_end];
        steps.push(JsonStep::Field {
            key: key.to_owned(),
            as_text,
        });
        rest = &after_arrow[key_end..];
    }

    (base, steps)
}

/// Errors from identifier / column-reference validation. Each maps to HTTP 400.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum IdentError {
    /// The identifier failed the safe-identifier character/shape check.
    #[error("unsafe identifier: {0}")]
    Unsafe(String),
    /// The cast target type failed validation.
    #[error("unsafe cast type: {0}")]
    UnsafeCast(String),
    /// An empty column reference was supplied.
    #[error("empty column reference")]
    Empty,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_column_round_trips() {
        let cr = parse_column_ref("age").expect("parse");
        assert_eq!(cr.base(), "age");
        assert_eq!(cr.to_sql(), "age");
    }

    #[test]
    fn json_text_and_object_paths() {
        assert_eq!(parse_column_ref("data->>name").unwrap().to_sql(), "data ->> 'name'");
        assert_eq!(parse_column_ref("data->addr").unwrap().to_sql(), "data -> 'addr'");
        assert_eq!(
            parse_column_ref("payload->addr->>city").unwrap().to_sql(),
            "payload -> 'addr' ->> 'city'"
        );
    }

    #[test]
    fn json_array_index_is_numeric() {
        assert_eq!(parse_column_ref("items->0").unwrap().to_sql(), "items -> 0");
        assert_eq!(parse_column_ref("items->>2").unwrap().to_sql(), "items ->> 2");
    }

    #[test]
    fn cast_is_validated_and_rendered() {
        assert_eq!(parse_column_ref("n::int").unwrap().to_sql(), "(n)::int");
        assert_eq!(
            parse_column_ref("data->>age::int").unwrap().to_sql(),
            "(data ->> 'age')::int"
        );
    }

    #[test]
    fn rejects_unsafe_base() {
        assert!(matches!(
            parse_column_ref("col; DROP TABLE users").unwrap_err(),
            IdentError::Unsafe(_)
        ));
    }

    #[test]
    fn rejects_unsafe_cast() {
        assert!(matches!(
            parse_column_ref("n::int; DROP").unwrap_err(),
            IdentError::UnsafeCast(_)
        ));
    }

    #[test]
    fn json_key_with_quote_is_escaped_not_injectable() {
        // A key containing a single quote must be doubled, keeping it inside the literal.
        let cr = parse_column_ref("data->>o'brien").unwrap();
        assert_eq!(cr.to_sql(), "data ->> 'o''brien'");
    }

    #[test]
    fn empty_reference_rejected() {
        assert!(matches!(parse_column_ref("").unwrap_err(), IdentError::Empty));
    }

    #[test]
    fn validate_identifier_accepts_qualified_names() {
        assert!(validate_identifier("public.orders").is_ok());
        assert!(validate_identifier("my_col_2").is_ok());
        assert!(validate_identifier("x; DROP").is_err());
        assert!(validate_identifier("").is_err());
    }
}
