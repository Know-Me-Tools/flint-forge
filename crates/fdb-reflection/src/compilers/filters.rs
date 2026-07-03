//! Bridge from PostgREST query parameters to the shared [`fdb_query`] translator.
//!
//! The reflection REST handlers used to carry their own filter parser and
//! WHERE-clause builder. That logic now lives in the pure `fdb-query` crate (one
//! authoritative, security-hardened translator shared with `fdb-postgres::PgRest`,
//! so the two REST paths cannot drift). This module only:
//!
//! 1. turns a query-parameter map into an `fdb_query::FilterTree` (AND of leaves,
//!    honoring `not.` and `op(any)`/`op(all)`), and
//! 2. renders that tree to a `WHERE …` fragment plus sqlx-bound parameters.
//!
//! Every identifier is validated and every value bound as `$n` inside `fdb-query`.

use std::collections::HashMap;

use fdb_query::{FilterTree, Operator, QueryParam, Quantifier};

/// Query-parameter keys that are NOT column filters (re-exported from `fdb-query`).
pub use fdb_query::RESERVED_PARAMS;

/// Error building a filter tree from query parameters. Maps to HTTP 400.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FilterError {
    /// A filter value was not `[not.]op[(any|all)].value`.
    #[error("malformed filter for `{0}`: expected `<op>.<value>`")]
    Malformed(String),
    /// The operator token was not recognized.
    #[error("unknown filter operator: {0}")]
    UnknownOp(String),
}

/// Parse the non-reserved query params into an `fdb_query::FilterTree` (AND of
/// leaves). Reserved keys (`select`/`order`/`limit`/`offset`) are skipped.
///
/// # Errors
/// Returns [`FilterError`] on a malformed `op.value` token or unknown operator.
pub fn parse_filter_tree(params: &HashMap<String, String>) -> Result<FilterTree, FilterError> {
    let mut leaves = Vec::new();
    for (key, raw) in params {
        if RESERVED_PARAMS.contains(&key.as_str()) {
            continue;
        }
        leaves.push(parse_leaf(key, raw)?);
    }
    Ok(FilterTree::And(leaves))
}

/// Parse one `column=[not.]op[(any|all)].value` pair into a leaf.
fn parse_leaf(column: &str, raw: &str) -> Result<FilterTree, FilterError> {
    let (negate, rest) = raw.strip_prefix("not.").map_or((false, raw), |r| (true, r));
    let (op_token, value) = rest
        .split_once('.')
        .ok_or_else(|| FilterError::Malformed(column.to_owned()))?;

    let (op_name, quantifier) = if let Some(base) = op_token.strip_suffix("(any)") {
        (base, Some(Quantifier::Any))
    } else if let Some(base) = op_token.strip_suffix("(all)") {
        (base, Some(Quantifier::All))
    } else {
        (op_token, None)
    };

    let op = Operator::parse(op_name).ok_or_else(|| FilterError::UnknownOp(op_name.to_owned()))?;
    Ok(FilterTree::Leaf {
        column: column.to_owned(),
        op,
        value: value.to_owned(),
        negate,
        quantifier,
    })
}

/// Render a filter tree to a `WHERE …` fragment (empty string when there are no
/// filters) plus its ordered bind parameters, numbering placeholders from
/// `start_index`.
///
/// The leading `WHERE ` keyword is included so callers can splice the fragment
/// directly (matching the previous `build_where` contract). An empty tree renders
/// to `""` with no binds.
///
/// # Errors
/// Returns a message string on identifier/render failure (maps to HTTP 400).
pub fn render_where(tree: &FilterTree, start_index: usize) -> Result<WhereClause, String> {
    let (sql, params, _) = tree.render(start_index).map_err(|e| e.to_string())?;
    // A top-level empty AND renders to "TRUE"; treat that as "no filter".
    if sql == "TRUE" {
        return Ok(WhereClause::default());
    }
    Ok(WhereClause {
        sql: format!("WHERE {sql}"),
        binds: params,
    })
}

/// A rendered `WHERE` clause plus its ordered bind values.
#[derive(Debug, Default, Clone)]
pub struct WhereClause {
    /// The `WHERE …` fragment (empty when there are no filters).
    pub sql: String,
    /// Bind values in `$n` order.
    pub binds: Vec<QueryParam>,
}

/// Bind an `fdb_query::QueryParam` onto a sqlx Postgres query builder.
///
/// `Text` binds as text, `TextArray` as `text[]`, `Json` as `jsonb`, `Null` as a
/// NULL text. Values never touch the SQL string — this is the parameter channel.
pub fn bind_param<'q>(
    q: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    param: &'q QueryParam,
) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
    match param {
        QueryParam::Text(s) => q.bind(s),
        QueryParam::TextArray(v) => q.bind(v),
        QueryParam::Json(s) => {
            let json: serde_json::Value =
                serde_json::from_str(s).unwrap_or_else(|_| serde_json::Value::String(s.clone()));
            q.bind(json)
        }
        // Null and any future (#[non_exhaustive]) variant bind as a NULL text.
        _ => q.bind(Option::<String>::None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect()
    }

    #[test]
    fn reserved_params_are_skipped() {
        let tree = parse_filter_tree(&params(&[("select", "id"), ("limit", "10")])).unwrap();
        // Only reserved keys → empty AND → renders to no WHERE.
        let wc = render_where(&tree, 1).unwrap();
        assert!(wc.sql.is_empty());
        assert!(wc.binds.is_empty());
    }

    #[test]
    fn single_filter_renders_where_with_keyword() {
        let tree = parse_filter_tree(&params(&[("status", "eq.active")])).unwrap();
        let wc = render_where(&tree, 1).unwrap();
        assert_eq!(wc.sql, "WHERE status = $1");
        assert_eq!(wc.binds, vec![QueryParam::Text("active".into())]);
    }

    #[test]
    fn multiple_filters_and_together_from_start_index() {
        let mut tree_params = params(&[("age", "gte.18")]);
        tree_params.insert("status".into(), "eq.active".into());
        let tree = parse_filter_tree(&tree_params).unwrap();
        let wc = render_where(&tree, 3).unwrap();
        // Two leaves AND-ed, parenthesized, numbered from $3.
        assert!(wc.sql.starts_with("WHERE ("));
        assert!(wc.sql.contains("$3") && wc.sql.contains("$4"));
        assert_eq!(wc.binds.len(), 2);
    }

    #[test]
    fn not_prefix_and_quantifier_parse() {
        let neg = parse_filter_tree(&params(&[("a", "not.eq.1")])).unwrap();
        assert_eq!(render_where(&neg, 1).unwrap().sql, "WHERE NOT (a = $1)");
        let quant = parse_filter_tree(&params(&[("id", "eq(any).(1,2)")])).unwrap();
        assert_eq!(render_where(&quant, 1).unwrap().sql, "WHERE id = ANY($1)");
    }

    #[test]
    fn unknown_operator_and_malformed_rejected() {
        assert!(matches!(
            parse_filter_tree(&params(&[("a", "bogus.1")])).unwrap_err(),
            FilterError::UnknownOp(_)
        ));
        assert!(matches!(
            parse_filter_tree(&params(&[("a", "novalue")])).unwrap_err(),
            FilterError::Malformed(_)
        ));
    }

    #[test]
    fn unsafe_column_rejected_at_render() {
        // Column safety is enforced by fdb-query at render time.
        let tree = parse_filter_tree(&params(&[("col; DROP", "eq.1")])).unwrap();
        assert!(render_where(&tree, 1).is_err());
    }
}
