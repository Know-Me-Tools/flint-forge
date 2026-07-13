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

use fdb_query::{CastHints, FilterTree, Operator, Quantifier, QueryParam};

use crate::model::{DatabaseModel, Table};

/// Query-parameter keys that are NOT column filters (re-exported from `fdb-query`).
pub use fdb_query::RESERVED_PARAMS;

/// Build the [`CastHints`] for one table from its reflected column types.
#[must_use]
pub fn cast_hints_for_table(table: &Table) -> CastHints {
    CastHints::from_pairs(table.columns.iter().map(|c| (c.name.clone(), &c.pg_type)))
}

/// Look up `schema.table` in `model` and build its [`CastHints`]. An empty
/// (no-op) hint set when the table is not found — filters/mutations against an
/// unreflected table already fail elsewhere; this never blocks that path.
#[must_use]
pub fn cast_hints_for(model: &DatabaseModel, schema: &str, table: &str) -> CastHints {
    model
        .tables
        .iter()
        .find(|t| t.schema == schema && t.name == table)
        .map_or_else(CastHints::none, cast_hints_for_table)
}

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
        fts_config: None,
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
    render_where_with_hints(tree, start_index, &CastHints::none())
}

/// As [`render_where`], but casting each leaf's bound placeholder to `hints`'s
/// resolved Postgres type for that leaf's base column, when one exists — the
/// fix for filtering a non-text column (`int4`/`int8`/`bool`/`uuid`/...): the
/// sqlx driver declares a bound `String` as `text` explicitly, so `id = $1`
/// against an `int4` column fails server-side without the `$1::int4` cast.
///
/// # Errors
/// Returns a message string on identifier/render failure (maps to HTTP 400).
pub fn render_where_with_hints(
    tree: &FilterTree,
    start_index: usize,
    hints: &CastHints,
) -> Result<WhereClause, String> {
    let (sql, params, _) = tree
        .render_with_hints(start_index, hints)
        .map_err(|e| e.to_string())?;
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

/// A JSON request-body value resolved to its SQL bind channel.
///
/// Mirrors [`QueryParam`]'s text-by-default binding for scalar values so a
/// mutation body value gets the same `$n::pg_type` cast treatment as a URL
/// filter value — `sqlx` declares a bound `String` as `text` explicitly, so
/// writing `5` into an `int4` column needs the same cast, not just filtering.
#[derive(Debug, Clone, PartialEq)]
pub enum MutationBind {
    /// A scalar value (string/number/bool), bound as text, optionally cast.
    Text(String),
    /// A JSON container (array/object) — the only sensible target is a
    /// `json`/`jsonb` column, so it binds unchanged, never cast.
    Json(serde_json::Value),
    /// `null` — binds SQL `NULL` (polymorphic; no cast needed).
    Null,
}

/// Convert a JSON body value into its bind channel. Scalars become PostgREST-
/// style text (numbers/bools via `to_string()`, strings verbatim, no JSON
/// quoting); containers stay JSON.
#[must_use]
pub fn mutation_value_to_bind(v: &serde_json::Value) -> MutationBind {
    match v {
        serde_json::Value::Null => MutationBind::Null,
        serde_json::Value::String(s) => MutationBind::Text(s.clone()),
        serde_json::Value::Number(n) => MutationBind::Text(n.to_string()),
        serde_json::Value::Bool(b) => MutationBind::Text(b.to_string()),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => MutationBind::Json(v.clone()),
    }
}

/// The `$n` placeholder for a mutation bind, cast to `cast` when the bind is
/// text and a cast is resolved (never for `Json`/`Null` — a container value
/// already targets `jsonb`, and casting `NULL` is unnecessary).
#[must_use]
pub fn mutation_placeholder(idx: usize, bind: &MutationBind, cast: Option<&str>) -> String {
    match (bind, cast) {
        (MutationBind::Text(_), Some(t)) => format!("${idx}::{t}"),
        _ => format!("${idx}"),
    }
}

/// Bind a [`MutationBind`] onto a sqlx Postgres query builder.
pub fn bind_mutation_value<'q>(
    q: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    bind: &'q MutationBind,
) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
    match bind {
        MutationBind::Text(s) => q.bind(s),
        MutationBind::Json(j) => q.bind(j),
        MutationBind::Null => q.bind(Option::<String>::None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
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

    #[test]
    fn render_where_with_hints_casts_typed_column() {
        let tree = parse_filter_tree(&params(&[("id", "eq.5")])).unwrap();
        let hints = CastHints::from_pairs([("id", "int4")]);
        let wc = render_where_with_hints(&tree, 1, &hints).unwrap();
        assert_eq!(wc.sql, "WHERE id = $1::int4");
    }

    #[test]
    fn mutation_value_to_bind_converts_json_scalars_to_text() {
        assert_eq!(
            mutation_value_to_bind(&serde_json::json!(5)),
            MutationBind::Text("5".into())
        );
        assert_eq!(
            mutation_value_to_bind(&serde_json::json!(true)),
            MutationBind::Text("true".into())
        );
        assert_eq!(
            mutation_value_to_bind(&serde_json::json!("alice")),
            MutationBind::Text("alice".into())
        );
        assert_eq!(
            mutation_value_to_bind(&serde_json::Value::Null),
            MutationBind::Null
        );
        assert!(matches!(
            mutation_value_to_bind(&serde_json::json!([1, 2])),
            MutationBind::Json(_)
        ));
    }

    #[test]
    fn mutation_placeholder_casts_only_text_binds() {
        assert_eq!(
            mutation_placeholder(1, &MutationBind::Text("5".into()), Some("int4")),
            "$1::int4"
        );
        assert_eq!(
            mutation_placeholder(1, &MutationBind::Text("5".into()), None),
            "$1"
        );
        assert_eq!(
            mutation_placeholder(1, &MutationBind::Null, Some("int4")),
            "$1",
            "NULL is never cast"
        );
        assert_eq!(
            mutation_placeholder(1, &MutationBind::Json(serde_json::json!([1])), Some("int4")),
            "$1",
            "JSON containers are never cast"
        );
    }
}
