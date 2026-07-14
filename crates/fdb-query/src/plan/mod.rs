//! Query-plan assembly and PostgREST request parsing.
//!
//! Turns a raw PostgREST query-parameter list (plus optional `Range`/`Prefer`
//! headers) into a typed [`SelectPlan`] that renders to `(sql, params)`. This is
//! the top-level entry the executor calls for reads.

mod parse;
#[cfg(test)]
mod tests;

use crate::clause::{CountStrategy, Limits, Order, Select};
use crate::filter::{FilterError, FilterTree};
use crate::ident::{validate_identifier, IdentError};
use crate::param::QueryParam;
use parse::{parse_count_from_prefer, parse_leaf, parse_logical_group};

/// Reserved query-parameter keys that are NOT column filters.
pub const RESERVED_PARAMS: &[&str] = &["select", "order", "limit", "offset"];

/// A fully parsed read query against a single relation.
#[derive(Debug, Clone)]
pub struct SelectPlan {
    /// Qualified relation, validated: `schema.table` or `table`.
    pub relation: String,
    pub select: Select,
    pub filter: FilterTree,
    pub order: Order,
    pub limits: Limits,
    pub count: CountStrategy,
}

impl SelectPlan {
    /// Render the read query to `(sql, params)`.
    ///
    /// # Errors
    /// Propagates [`FilterError`] from the WHERE tree.
    pub fn render(&self) -> Result<(String, Vec<QueryParam>), FilterError> {
        use std::fmt::Write as _;
        let (where_sql, params, _) = self.filter.render(1)?;
        let mut sql = format!("SELECT {} FROM {}", self.select.to_sql(), self.relation);
        // A top-level empty AND renders "TRUE"; skip the WHERE in that case.
        if where_sql != "TRUE" {
            let _ = write!(sql, " WHERE {where_sql}");
        }
        if !self.order.is_empty() {
            sql.push(' ');
            sql.push_str(&self.order.to_sql());
        }
        sql.push_str(&self.limits.to_sql());
        Ok((sql, params))
    }
}

/// Parse a PostgREST read request into a [`SelectPlan`].
///
/// `relation` is the target relation (validated). `params` is the query-string
/// key/value list (duplicate keys allowed and preserved — e.g. repeated column
/// filters AND together). `range` and `prefer` are the optional headers.
///
/// # Errors
/// Returns [`ParseError`] for an unsafe relation, malformed clause, or bad filter.
pub fn parse_select_request(
    relation: &str,
    params: &[(String, String)],
    range: Option<&str>,
    prefer: Option<&str>,
) -> Result<SelectPlan, ParseError> {
    let relation = validate_identifier(relation)
        .map_err(|_| ParseError::UnsafeRelation(relation.to_owned()))?
        .to_owned();

    let mut select = Select::default();
    let mut order = Order::default();
    let mut limit: Option<u64> = None;
    let mut offset: Option<u64> = None;
    let mut leaves: Vec<FilterTree> = Vec::new();

    for (key, value) in params {
        match key.as_str() {
            "select" => select = Select::parse(value)?,
            "order" => order = Order::parse(value).map_err(ParseError::from)?,
            "limit" => {
                limit = Some(
                    value
                        .parse()
                        .map_err(|_| ParseError::BadNumber(value.clone()))?,
                );
            }
            "offset" => {
                offset = Some(
                    value
                        .parse()
                        .map_err(|_| ParseError::BadNumber(value.clone()))?,
                );
            }
            "and" => leaves.push(parse_logical_group("and", value)?),
            "or" => leaves.push(parse_logical_group("or", value)?),
            "not.and" => leaves.push(FilterTree::Not(Box::new(parse_logical_group(
                "and", value,
            )?))),
            "not.or" => leaves.push(FilterTree::Not(Box::new(parse_logical_group("or", value)?))),
            _ => leaves.push(parse_leaf(key, value)?),
        }
    }

    // Range header overrides limit/offset params when present.
    let limits = if let Some(r) = range {
        Limits::from_range_header(r).map_err(|e| ParseError::BadRange(e.to_string()))?
    } else {
        Limits::from_params(limit, offset)
    };

    let count = prefer.map_or(CountStrategy::None, parse_count_from_prefer);

    Ok(SelectPlan {
        relation,
        select,
        filter: FilterTree::And(leaves),
        order,
        limits,
        count,
    })
}

/// Errors from request parsing. Each maps to HTTP 400.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    #[error("unsafe relation: {0}")]
    UnsafeRelation(String),
    #[error(transparent)]
    Ident(#[from] IdentError),
    #[error(transparent)]
    Filter(#[from] FilterError),
    #[error(transparent)]
    Order(#[from] crate::clause::OrderError),
    #[error("malformed filter for `{0}`: expected `<op>.<value>`")]
    MalformedFilter(String),
    #[error("malformed logical group: {0}")]
    MalformedGroup(String),
    #[error("unknown operator: {0}")]
    UnknownOp(String),
    #[error("invalid number: {0}")]
    BadNumber(String),
    #[error("invalid Range header: {0}")]
    BadRange(String),
}
