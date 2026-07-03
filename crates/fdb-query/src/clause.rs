//! Vertical filtering (`select`), ordering, pagination, and count strategy.
//!
//! These are the non-WHERE parts of a read query. Each parses from its PostgREST
//! query-parameter / header form into a typed value and renders a safe SQL
//! fragment. Identifiers go through [`crate::ident`]; nothing user-controlled is
//! interpolated unvalidated.

use crate::ident::{IdentError, parse_column_ref, validate_identifier};

/// A `select` projection: a list of output columns, each optionally renamed.
///
/// `select=id,full_name:name,data->>email` →
/// `id, name AS full_name, data ->> 'email'`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Select {
    items: Vec<SelectItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SelectItem {
    /// Output alias (`alias:` prefix), validated.
    alias: Option<String>,
    /// The source column reference (validated), rendered SQL.
    expr_sql: String,
}

impl Select {
    /// Parse a `select` parameter value.
    ///
    /// # Errors
    /// Returns [`IdentError`] when an alias or column reference fails validation.
    pub fn parse(raw: &str) -> Result<Self, IdentError> {
        let mut items = Vec::new();
        for token in split_top_level_commas(raw) {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            // `alias:column` — the alias is before the first ':' that is not part
            // of a `::cast`. We look for a single ':' not followed by another ':'.
            let (alias, col_part) = split_alias(token);
            let alias = match alias {
                Some(a) => Some(validate_identifier(a).map_err(|_| IdentError::Unsafe(a.to_owned()))?.to_owned()),
                None => None,
            };
            let expr_sql = parse_column_ref(col_part)?.to_sql();
            items.push(SelectItem { alias, expr_sql });
        }
        Ok(Self { items })
    }

    /// Render the projection list. Empty projection renders as `*`.
    #[must_use]
    pub fn to_sql(&self) -> String {
        if self.items.is_empty() {
            return "*".to_owned();
        }
        self.items
            .iter()
            .map(|it| match &it.alias {
                Some(a) => format!("{} AS {a}", it.expr_sql),
                None => it.expr_sql.clone(),
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Split an `alias:column` token. Returns `(Some(alias), column)` when a single
/// leading `:` separator (not a `::` cast) is present, else `(None, token)`.
fn split_alias(token: &str) -> (Option<&str>, &str) {
    let bytes = token.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b':' {
            // A `::` is a cast, not an alias separator — skip both.
            if bytes.get(i + 1) == Some(&b':') {
                i += 2;
                continue;
            }
            return (Some(&token[..i]), &token[i + 1..]);
        }
        i += 1;
    }
    (None, token)
}

/// Ordering: a list of `column[.asc|.desc][.nullsfirst|.nullslast]` terms.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Order {
    terms: Vec<OrderTerm>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OrderTerm {
    expr_sql: String,
    descending: bool,
    nulls: Option<NullsPos>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NullsPos {
    First,
    Last,
}

impl Order {
    /// Parse an `order` parameter value: `col1.desc.nullslast,col2.asc`.
    ///
    /// # Errors
    /// Returns [`IdentError`] when a column reference fails validation, or
    /// [`OrderError`] for an unrecognized direction/nulls modifier.
    pub fn parse(raw: &str) -> Result<Self, OrderError> {
        let mut terms = Vec::new();
        for token in split_top_level_commas(raw) {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            let mut parts = token.split('.');
            let col = parts.next().unwrap_or("");
            let expr_sql = parse_column_ref(col)?.to_sql();
            let mut descending = false;
            let mut nulls = None;
            for modifier in parts {
                match modifier {
                    "asc" => descending = false,
                    "desc" => descending = true,
                    "nullsfirst" => nulls = Some(NullsPos::First),
                    "nullslast" => nulls = Some(NullsPos::Last),
                    other => return Err(OrderError::BadModifier(other.to_owned())),
                }
            }
            terms.push(OrderTerm {
                expr_sql,
                descending,
                nulls,
            });
        }
        Ok(Self { terms })
    }

    /// True when there are no ordering terms (caller omits the `ORDER BY`).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// Render `ORDER BY ...` (empty string when there are no terms).
    #[must_use]
    pub fn to_sql(&self) -> String {
        if self.terms.is_empty() {
            return String::new();
        }
        let parts: Vec<String> = self
            .terms
            .iter()
            .map(|t| {
                let dir = if t.descending { " DESC" } else { " ASC" };
                let nulls = match t.nulls {
                    Some(NullsPos::First) => " NULLS FIRST",
                    Some(NullsPos::Last) => " NULLS LAST",
                    None => "",
                };
                format!("{}{dir}{nulls}", t.expr_sql)
            })
            .collect();
        format!("ORDER BY {}", parts.join(", "))
    }
}

/// Errors from `order` parsing.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum OrderError {
    /// A column reference failed validation.
    #[error(transparent)]
    Ident(#[from] IdentError),
    /// An order modifier was not asc/desc/nullsfirst/nullslast.
    #[error("invalid order modifier: {0}")]
    BadModifier(String),
}

/// Pagination bounds resolved from `limit`/`offset` params or a `Range` header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Limits {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

impl Limits {
    /// Build from explicit `limit`/`offset` query params.
    #[must_use]
    pub fn from_params(limit: Option<u64>, offset: Option<u64>) -> Self {
        Self { limit, offset }
    }

    /// Parse an HTTP `Range` header of the form `0-24` (inclusive), unit `items`.
    /// `Range: 0-24` → offset 0, limit 25.
    ///
    /// # Errors
    /// Returns [`RangeError`] when the header is not `start-end` with `start <= end`.
    pub fn from_range_header(range: &str) -> Result<Self, RangeError> {
        let (start, end) = range
            .split_once('-')
            .ok_or_else(|| RangeError::Malformed(range.to_owned()))?;
        let start: u64 = start.trim().parse().map_err(|_| RangeError::Malformed(range.to_owned()))?;
        if end.trim().is_empty() {
            // Open-ended range `start-`: offset only.
            return Ok(Self {
                limit: None,
                offset: Some(start),
            });
        }
        let end: u64 = end.trim().parse().map_err(|_| RangeError::Malformed(range.to_owned()))?;
        if end < start {
            return Err(RangeError::Inverted { start, end });
        }
        Ok(Self {
            limit: Some(end - start + 1),
            offset: Some(start),
        })
    }

    /// Render ` LIMIT n OFFSET m` fragments (each omitted when unset).
    #[must_use]
    pub fn to_sql(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        if let Some(l) = self.limit {
            let _ = write!(out, " LIMIT {l}");
        }
        if let Some(o) = self.offset {
            let _ = write!(out, " OFFSET {o}");
        }
        out
    }
}

/// Errors from `Range` header parsing.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RangeError {
    /// The range was not `start-end`.
    #[error("malformed Range: {0}")]
    Malformed(String),
    /// `end` was less than `start`.
    #[error("inverted Range: {start}-{end}")]
    Inverted { start: u64, end: u64 },
}

/// The `count` strategy from `Prefer: count=exact|planned|estimated`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CountStrategy {
    /// No count requested.
    None,
    /// `count(*)` over the filtered set — exact but scans.
    Exact,
    /// Planner row estimate (`EXPLAIN`), cheap.
    Planned,
    /// Larger of exact/planned per PostgREST semantics.
    Estimated,
}

impl CountStrategy {
    /// Parse the `count=` value from a `Prefer` header.
    #[must_use]
    pub fn parse(value: &str) -> Self {
        match value {
            "exact" => Self::Exact,
            "planned" => Self::Planned,
            "estimated" => Self::Estimated,
            _ => Self::None,
        }
    }
}

/// Split on commas that are not inside parentheses (so embedded resources and
/// quoted lists survive tokenization intact).
pub(crate) fn split_top_level_commas(raw: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (i, c) in raw.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                out.push(&raw[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&raw[start..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_default_is_star() {
        assert_eq!(Select::default().to_sql(), "*");
        assert_eq!(Select::parse("").unwrap().to_sql(), "*");
    }

    #[test]
    fn select_columns_rename_and_json() {
        let s = Select::parse("id,full_name:name,data->>email").unwrap();
        assert_eq!(s.to_sql(), "id, name AS full_name, data ->> 'email'");
    }

    #[test]
    fn select_with_cast_not_confused_with_alias() {
        // `n::int` is a cast, not `n` aliased to `:int`.
        let s = Select::parse("n::int").unwrap();
        assert_eq!(s.to_sql(), "(n)::int");
    }

    #[test]
    fn select_rejects_unsafe() {
        assert!(Select::parse("id; DROP").is_err());
    }

    #[test]
    fn order_directions_and_nulls() {
        let o = Order::parse("created_at.desc.nullslast,name.asc").unwrap();
        assert_eq!(o.to_sql(), "ORDER BY created_at DESC NULLS LAST, name ASC");
    }

    #[test]
    fn order_default_asc() {
        assert_eq!(Order::parse("id").unwrap().to_sql(), "ORDER BY id ASC");
    }

    #[test]
    fn order_bad_modifier_errors() {
        assert!(matches!(
            Order::parse("id.sideways").unwrap_err(),
            OrderError::BadModifier(_)
        ));
    }

    #[test]
    fn order_empty_renders_nothing() {
        assert!(Order::default().is_empty());
        assert_eq!(Order::default().to_sql(), "");
    }

    #[test]
    fn limits_from_params() {
        let l = Limits::from_params(Some(10), Some(20));
        assert_eq!(l.to_sql(), " LIMIT 10 OFFSET 20");
    }

    #[test]
    fn limits_zero_is_valid() {
        assert_eq!(Limits::from_params(Some(0), None).to_sql(), " LIMIT 0");
    }

    #[test]
    fn range_header_inclusive() {
        let l = Limits::from_range_header("0-24").unwrap();
        assert_eq!(l, Limits { limit: Some(25), offset: Some(0) });
        let l = Limits::from_range_header("10-19").unwrap();
        assert_eq!(l, Limits { limit: Some(10), offset: Some(10) });
    }

    #[test]
    fn range_open_ended_offset_only() {
        let l = Limits::from_range_header("50-").unwrap();
        assert_eq!(l, Limits { limit: None, offset: Some(50) });
    }

    #[test]
    fn range_inverted_and_malformed_error() {
        assert!(matches!(
            Limits::from_range_header("24-0").unwrap_err(),
            RangeError::Inverted { .. }
        ));
        assert!(matches!(
            Limits::from_range_header("garbage").unwrap_err(),
            RangeError::Malformed(_)
        ));
    }

    #[test]
    fn count_strategy_parse() {
        assert_eq!(CountStrategy::parse("exact"), CountStrategy::Exact);
        assert_eq!(CountStrategy::parse("planned"), CountStrategy::Planned);
        assert_eq!(CountStrategy::parse("estimated"), CountStrategy::Estimated);
        assert_eq!(CountStrategy::parse("nope"), CountStrategy::None);
    }

    #[test]
    fn top_level_comma_split_respects_parens() {
        assert_eq!(split_top_level_commas("a,b(c,d),e"), vec!["a", "b(c,d)", "e"]);
    }
}
