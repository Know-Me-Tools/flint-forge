//! PostgREST-style filter parsing and parameterised WHERE-clause building for
//! the REST `handle_list` handler.
//!
//! Query parameters use the form `?<column>=<op>.<value>`, e.g.
//! `?age=gte.18&status=eq.active`. Column names are validated with
//! [`forge_domain::is_safe_identifier`] before they reach SQL; values are
//! **always** bound as `$1`, `$2`, … and never interpolated.

use forge_domain::is_safe_identifier;

/// The 12 supported filter operators (PostgREST-compatible subset).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FilterOp {
    /// `eq`  → `=`
    Eq,
    /// `neq` → `<>`
    Neq,
    /// `gt`  → `>`
    Gt,
    /// `gte` → `>=`
    Gte,
    /// `lt`  → `<`
    Lt,
    /// `lte` → `<=`
    Lte,
    /// `like`  → `LIKE`
    Like,
    /// `ilike` → `ILIKE`
    Ilike,
    /// `in`  → `= ANY(...)` over a comma list
    In,
    /// `is`  → `IS` (null / true / false)
    Is,
    /// `cs`  → `@>` (contains)
    Cs,
    /// `cd`  → `<@` (contained by)
    Cd,
}

impl FilterOp {
    /// Parse the operator token (the part before the first `.`).
    fn parse(token: &str) -> Option<Self> {
        Some(match token {
            "eq" => Self::Eq,
            "neq" => Self::Neq,
            "gt" => Self::Gt,
            "gte" => Self::Gte,
            "lt" => Self::Lt,
            "lte" => Self::Lte,
            "like" => Self::Like,
            "ilike" => Self::Ilike,
            "in" => Self::In,
            "is" => Self::Is,
            "cs" => Self::Cs,
            "cd" => Self::Cd,
            _ => return None,
        })
    }

    /// SQL infix operator for the scalar/binary operators. Returns `None` for
    /// operators with bespoke rendering (`In`, `Is`).
    fn sql_infix(self) -> Option<&'static str> {
        Some(match self {
            Self::Eq => "=",
            Self::Neq => "<>",
            Self::Gt => ">",
            Self::Gte => ">=",
            Self::Lt => "<",
            Self::Lte => "<=",
            Self::Like => "LIKE",
            Self::Ilike => "ILIKE",
            Self::Cs => "@>",
            Self::Cd => "<@",
            Self::In | Self::Is => return None,
        })
    }
}

/// A single parsed filter: `<column> <op> <raw value>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    pub column: String,
    pub op: FilterOp,
    /// The raw value token (after the first `.`); interpretation depends on `op`.
    pub value: String,
}

/// Errors from filter parsing. Each maps to an HTTP 400.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FilterError {
    #[error("unsafe column identifier: {0}")]
    UnsafeColumn(String),
    #[error("malformed filter for `{0}`: expected `<op>.<value>`")]
    Malformed(String),
    #[error("unknown filter operator: {0}")]
    UnknownOp(String),
    #[error("invalid `is` value: {0} (expected null/true/false)")]
    InvalidIs(String),
}

/// Parse one `column=op.value` pair into a [`Filter`].
///
/// The column name is validated up front; the operator token is everything
/// before the first `.`, and the value is the (possibly empty, possibly
/// dot-containing) remainder.
pub fn parse_filter(column: &str, raw: &str) -> Result<Filter, FilterError> {
    if !is_safe_identifier(column) {
        return Err(FilterError::UnsafeColumn(column.to_owned()));
    }
    let (op_token, value) = raw
        .split_once('.')
        .ok_or_else(|| FilterError::Malformed(column.to_owned()))?;
    let op = FilterOp::parse(op_token)
        .ok_or_else(|| FilterError::UnknownOp(op_token.to_owned()))?;
    if op == FilterOp::Is {
        // Validate the `is` operand eagerly so it renders as a literal safely.
        match value.to_ascii_lowercase().as_str() {
            "null" | "true" | "false" => {}
            other => return Err(FilterError::InvalidIs(other.to_owned())),
        }
    }
    Ok(Filter {
        column: column.to_owned(),
        op,
        value: value.to_owned(),
    })
}

/// The reserved query-parameter keys that are NOT filters.
pub const RESERVED_PARAMS: &[&str] = &["select", "order", "limit", "offset"];

/// A rendered WHERE clause plus its ordered bind values.
#[derive(Debug, Default, Clone)]
pub struct WhereClause {
    /// The `WHERE ...` fragment (empty string when there are no filters).
    pub sql: String,
    /// Bind values in `$n` order.
    pub binds: Vec<String>,
}

/// Build a parameterised `WHERE` clause from parsed filters.
///
/// `start_index` is the first placeholder number to use (so callers that have
/// already bound parameters can continue the sequence). Every value is bound;
/// only the (already-validated) column name is interpolated.
#[must_use]
pub fn build_where(filters: &[Filter], start_index: usize) -> WhereClause {
    if filters.is_empty() {
        return WhereClause::default();
    }

    let mut conditions: Vec<String> = Vec::with_capacity(filters.len());
    let mut binds: Vec<String> = Vec::new();
    let mut idx = start_index;

    for f in filters {
        let col = &f.column;
        match f.op {
            FilterOp::Is => {
                // Value is validated to null/true/false in parse_filter — safe to inline.
                let literal = f.value.to_ascii_uppercase();
                conditions.push(format!("{col} IS {literal}"));
            }
            FilterOp::In => {
                // `col IN (a,b,c)` → `col = ANY($n)` with a text[] bind.
                // Split on commas; bind the whole array as one parameter.
                let items: Vec<String> =
                    f.value.split(',').map(str::to_owned).collect();
                conditions.push(format!("{col} = ANY(${idx})"));
                // Encode as a Postgres array literal in a single text bind.
                binds.push(pg_text_array(&items));
                idx += 1;
            }
            other => {
                let infix = other
                    .sql_infix()
                    .expect("non In/Is operator always has an infix");
                conditions.push(format!("{col} {infix} ${idx}"));
                binds.push(f.value.clone());
                idx += 1;
            }
        }
    }

    WhereClause {
        sql: format!("WHERE {}", conditions.join(" AND ")),
        binds,
    }
}

/// Encode a list of strings as a Postgres array literal, e.g. `{a,b,c}`.
/// Each element is quoted and internal quotes/backslashes are escaped so the
/// literal is safe to bind as a single `text[]`-castable parameter.
fn pg_text_array(items: &[String]) -> String {
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
    fn parses_each_of_the_twelve_operators() {
        let cases = [
            ("eq.1", FilterOp::Eq),
            ("neq.1", FilterOp::Neq),
            ("gt.1", FilterOp::Gt),
            ("gte.1", FilterOp::Gte),
            ("lt.1", FilterOp::Lt),
            ("lte.1", FilterOp::Lte),
            ("like.a%", FilterOp::Like),
            ("ilike.a%", FilterOp::Ilike),
            ("in.a,b,c", FilterOp::In),
            ("is.null", FilterOp::Is),
            ("cs.{1,2}", FilterOp::Cs),
            ("cd.{1,2}", FilterOp::Cd),
        ];
        for (raw, expected) in cases {
            let f = parse_filter("col", raw).expect("should parse");
            assert_eq!(f.op, expected, "raw={raw}");
        }
    }

    #[test]
    fn rejects_unsafe_column() {
        let err = parse_filter("col; DROP", "eq.1").unwrap_err();
        assert!(matches!(err, FilterError::UnsafeColumn(_)));
    }

    #[test]
    fn rejects_malformed_and_unknown_op() {
        assert!(matches!(
            parse_filter("col", "novalue").unwrap_err(),
            FilterError::Malformed(_)
        ));
        assert!(matches!(
            parse_filter("col", "bogus.1").unwrap_err(),
            FilterError::UnknownOp(_)
        ));
    }

    #[test]
    fn rejects_invalid_is_operand() {
        assert!(matches!(
            parse_filter("col", "is.maybe").unwrap_err(),
            FilterError::InvalidIs(_)
        ));
        assert!(parse_filter("col", "is.null").is_ok());
        assert!(parse_filter("col", "is.TRUE").is_ok());
    }

    #[test]
    fn build_where_scalar_operators_are_parameterised() {
        let filters = vec![
            parse_filter("age", "gte.18").unwrap(),
            parse_filter("status", "eq.active").unwrap(),
        ];
        let wc = build_where(&filters, 1);
        assert_eq!(wc.sql, "WHERE age >= $1 AND status = $2");
        assert_eq!(wc.binds, vec!["18".to_owned(), "active".to_owned()]);
    }

    #[test]
    fn build_where_respects_start_index() {
        let filters = vec![parse_filter("age", "gt.21").unwrap()];
        let wc = build_where(&filters, 3);
        assert_eq!(wc.sql, "WHERE age > $3");
    }

    #[test]
    fn build_where_is_operator_inlines_validated_literal() {
        let filters = vec![parse_filter("deleted_at", "is.null").unwrap()];
        let wc = build_where(&filters, 1);
        assert_eq!(wc.sql, "WHERE deleted_at IS NULL");
        assert!(wc.binds.is_empty(), "IS binds nothing");
    }

    #[test]
    fn build_where_in_operator_uses_any_with_array_bind() {
        let filters = vec![parse_filter("id", "in.1,2,3").unwrap()];
        let wc = build_where(&filters, 1);
        assert_eq!(wc.sql, "WHERE id = ANY($1)");
        assert_eq!(wc.binds, vec![r#"{"1","2","3"}"#.to_owned()]);
    }

    #[test]
    fn build_where_containment_operators_render_correctly() {
        let cs = build_where(&[parse_filter("tags", "cs.{a}").unwrap()], 1);
        assert_eq!(cs.sql, "WHERE tags @> $1");
        let cd = build_where(&[parse_filter("tags", "cd.{a,b}").unwrap()], 1);
        assert_eq!(cd.sql, "WHERE tags <@ $1");
    }

    #[test]
    fn empty_filters_produce_empty_clause() {
        let wc = build_where(&[], 1);
        assert!(wc.sql.is_empty());
        assert!(wc.binds.is_empty());
    }

    /// Each of the 12 operators renders to its exact SQL fragment.
    #[test]
    fn every_operator_renders_expected_sql() {
        let cases: &[(&str, &str)] = &[
            ("eq.1", "WHERE c = $1"),
            ("neq.1", "WHERE c <> $1"),
            ("gt.1", "WHERE c > $1"),
            ("gte.1", "WHERE c >= $1"),
            ("lt.1", "WHERE c < $1"),
            ("lte.1", "WHERE c <= $1"),
            ("like.a%", "WHERE c LIKE $1"),
            ("ilike.a%", "WHERE c ILIKE $1"),
            ("in.1,2", "WHERE c = ANY($1)"),
            ("is.null", "WHERE c IS NULL"),
            ("cs.{1}", "WHERE c @> $1"),
            ("cd.{1}", "WHERE c <@ $1"),
        ];
        for (raw, expected_sql) in cases {
            let f = parse_filter("c", raw).expect("parse");
            let wc = build_where(&[f], 1);
            assert_eq!(&wc.sql, expected_sql, "raw={raw}");
        }
    }
}
