//! Query-plan assembly and PostgREST request parsing.
//!
//! Turns a raw PostgREST query-parameter list (plus optional `Range`/`Prefer`
//! headers) into a typed [`SelectPlan`] that renders to `(sql, params)`. This is
//! the top-level entry the executor calls for reads.

use crate::clause::{CountStrategy, Limits, Order, Select};
use crate::filter::{FilterError, FilterTree};
use crate::fts::FtsConfig;
use crate::ident::{IdentError, validate_identifier};
use crate::operator::{Operator, Quantifier, RenderError};
use crate::param::QueryParam;

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
                limit = Some(value.parse().map_err(|_| ParseError::BadNumber(value.clone()))?);
            }
            "offset" => {
                offset = Some(value.parse().map_err(|_| ParseError::BadNumber(value.clone()))?);
            }
            "and" => leaves.push(parse_logical_group("and", value)?),
            "or" => leaves.push(parse_logical_group("or", value)?),
            "not.and" => leaves.push(FilterTree::Not(Box::new(parse_logical_group("and", value)?))),
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

/// Parse a single `column=value` filter into a leaf, honoring `not.` and
/// `op(any)`/`op(all)` modifiers. Value form: `[not.]op[(any|all)].operand`.
fn parse_leaf(column: &str, raw: &str) -> Result<FilterTree, ParseError> {
    let (negate, rest) = match raw.strip_prefix("not.") {
        Some(r) => (true, r),
        None => (false, raw),
    };
    let (op_token, value) = rest
        .split_once('.')
        .ok_or_else(|| ParseError::MalformedFilter(column.to_owned()))?;

    let (op, quantifier, fts_config) = parse_op_token(op_token)?;

    Ok(FilterTree::Leaf {
        column: column.to_owned(),
        op,
        value: value.to_owned(),
        negate,
        quantifier,
        fts_config,
    })
}

/// Parse an operator token that may carry an optional `(payload)` suffix, e.g.
/// `eq`, `eq(any)`, `like(all)`, `fts`, `fts(english)`.
///
/// The suffix is interpreted by the *parsed operator kind*, not by its syntax:
/// for the four FTS operators the payload is a text-search [`FtsConfig`]; for all
/// other operators it is an `(any)`/`(all)` [`Quantifier`]. This keeps
/// `fts(english)` and `eq(any)` unambiguous. Exactly one of the returned options
/// is ever `Some`.
///
/// # Errors
/// Returns [`ParseError::UnknownOp`] for an unrecognized operator,
/// [`ParseError::MalformedFilter`] for a non-FTS operator carrying a payload that
/// is not `(any)`/`(all)`, or a wrapped [`RenderError::InvalidFtsConfig`] when an
/// FTS config fails identifier validation.
fn parse_op_token(
    op_token: &str,
) -> Result<(Operator, Option<Quantifier>, Option<FtsConfig>), ParseError> {
    // Split an optional trailing `(payload)` generically, before deciding meaning.
    let (base, payload) = match op_token.strip_suffix(')').and_then(|s| s.split_once('(')) {
        Some((base, payload)) => (base, Some(payload)),
        None => (op_token, None),
    };

    let op = Operator::parse(base).ok_or_else(|| ParseError::UnknownOp(base.to_owned()))?;

    if op.fts_kind().is_some() {
        // FTS operator: the payload (if any) is a text-search config.
        let cfg = payload
            .map(FtsConfig::parse)
            .transpose()
            .map_err(|e| ParseError::Filter(FilterError::Render(RenderError::InvalidFtsConfig(
                e.to_string(),
            ))))?;
        return Ok((op, None, cfg));
    }

    // Non-FTS operator: the payload (if any) is an `(any)`/`(all)` quantifier.
    let quantifier = match payload {
        Some("any") => Some(Quantifier::Any),
        Some("all") => Some(Quantifier::All),
        Some(_) => return Err(ParseError::MalformedFilter(op_token.to_owned())),
        None => None,
    };
    Ok((op, quantifier, None))
}

/// Parse a logical group value `(cond,cond,...)` into an And/Or node. Each member
/// is either a nested `and(...)`/`or(...)` group or a `column.op.value` triple.
fn parse_logical_group(kind: &str, raw: &str) -> Result<FilterTree, ParseError> {
    let inner = raw
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| ParseError::MalformedGroup(raw.to_owned()))?;

    let members = crate::clause::split_top_level_commas(inner);
    let mut children = Vec::with_capacity(members.len());
    for member in members {
        let member = member.trim();
        if member.is_empty() {
            continue;
        }
        // Only treat a member as a nested group when the connective keyword is
        // immediately followed by `(` (`and(...)` / `or(...)`). A column that merely
        // *starts* with the text `and`/`or` (e.g. `android.eq.1`, `origin.eq.x`) is a
        // leaf, not a group — `split_group_prefix` enforces that boundary.
        if let Some((connective, sub)) = split_group_prefix(member) {
            children.push(parse_logical_group(connective, sub)?);
        } else {
            children.push(parse_group_member(member)?);
        }
    }
    Ok(match kind {
        "or" => FilterTree::Or(children),
        _ => FilterTree::And(children),
    })
}

/// Detect a nested logical-group member. Returns `(connective, "(...)")` only when
/// `member` begins with the connective keyword *immediately followed by* `(` — i.e.
/// `and(` or `or(`. Otherwise returns `None` so the member is parsed as a leaf,
/// preventing a spurious group match for columns like `android` or `origin`.
fn split_group_prefix(member: &str) -> Option<(&'static str, &str)> {
    if let Some(rest) = member.strip_prefix("and") {
        if rest.starts_with('(') {
            return Some(("and", rest));
        }
    }
    if let Some(rest) = member.strip_prefix("or") {
        if rest.starts_with('(') {
            return Some(("or", rest));
        }
    }
    None
}

/// A group member is `column.op.value` (dotted form inside a logical group),
/// with optional leading `not.`.
fn parse_group_member(member: &str) -> Result<FilterTree, ParseError> {
    let (negate, rest) = match member.strip_prefix("not.") {
        Some(r) => (true, r),
        None => (false, member),
    };
    // column is up to the first '.', then op, then value (value may contain dots).
    let (column, after) = rest
        .split_once('.')
        .ok_or_else(|| ParseError::MalformedGroup(member.to_owned()))?;
    let (op_token, value) = after
        .split_once('.')
        .ok_or_else(|| ParseError::MalformedGroup(member.to_owned()))?;

    let (op, quantifier, fts_config) = parse_op_token(op_token)?;

    Ok(FilterTree::Leaf {
        column: column.to_owned(),
        op,
        value: value.to_owned(),
        negate,
        quantifier,
        fts_config,
    })
}

/// Extract the `count=` strategy from a `Prefer` header value (comma/space list).
fn parse_count_from_prefer(prefer: &str) -> CountStrategy {
    for part in prefer.split([',', ' ']) {
        if let Some(v) = part.trim().strip_prefix("count=") {
            return CountStrategy::parse(v);
        }
    }
    CountStrategy::None
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

#[cfg(test)]
mod tests {
    use super::*;

    fn p(k: &str, v: &str) -> (String, String) {
        (k.to_owned(), v.to_owned())
    }

    #[test]
    fn simple_filters_and_together() {
        let plan = parse_select_request(
            "orders",
            &[p("status", "eq.active"), p("total", "gte.100")],
            None,
            None,
        )
        .unwrap();
        let (sql, params) = plan.render().unwrap();
        assert_eq!(
            sql,
            "SELECT * FROM orders WHERE (status = $1 AND total >= $2)"
        );
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn select_order_limit_offset_compose() {
        let plan = parse_select_request(
            "users",
            &[
                p("select", "id,email:mail"),
                p("order", "created_at.desc"),
                p("limit", "10"),
                p("offset", "5"),
                p("active", "is.true"),
            ],
            None,
            None,
        )
        .unwrap();
        let (sql, _) = plan.render().unwrap();
        assert_eq!(
            sql,
            "SELECT id, mail AS email FROM users WHERE active IS TRUE ORDER BY created_at DESC LIMIT 10 OFFSET 5"
        );
    }

    #[test]
    fn no_filters_omits_where() {
        let plan = parse_select_request("t", &[], None, None).unwrap();
        assert_eq!(plan.render().unwrap().0, "SELECT * FROM t");
    }

    #[test]
    fn not_prefix_negates_leaf() {
        let plan = parse_select_request("t", &[p("a", "not.eq.1")], None, None).unwrap();
        assert_eq!(plan.render().unwrap().0, "SELECT * FROM t WHERE NOT (a = $1)");
    }

    #[test]
    fn quantifier_suffix_on_leaf() {
        let plan = parse_select_request("t", &[p("id", "eq(any).(1,2,3)")], None, None).unwrap();
        let (sql, params) = plan.render().unwrap();
        assert_eq!(sql, "SELECT * FROM t WHERE id = ANY($1)");
        assert_eq!(params, vec![QueryParam::TextArray(vec!["1".into(), "2".into(), "3".into()])]);
    }

    #[test]
    fn logical_or_group() {
        let plan = parse_select_request("t", &[p("or", "(a.eq.1,b.eq.2)")], None, None).unwrap();
        assert_eq!(plan.render().unwrap().0, "SELECT * FROM t WHERE (a = $1 OR b = $2)");
    }

    #[test]
    fn nested_and_or_group() {
        let plan = parse_select_request("t", &[p("and", "(a.gt.1,or(b.eq.2,c.eq.3))")], None, None)
            .unwrap();
        assert_eq!(
            plan.render().unwrap().0,
            "SELECT * FROM t WHERE (a > $1 AND (b = $2 OR c = $3))"
        );
    }

    #[test]
    fn not_and_group_negates() {
        let plan = parse_select_request("t", &[p("not.and", "(a.gte.0,a.lte.9)")], None, None)
            .unwrap();
        assert_eq!(
            plan.render().unwrap().0,
            "SELECT * FROM t WHERE NOT ((a >= $1 AND a <= $2))"
        );
    }

    #[test]
    fn range_header_overrides_limit() {
        let plan = parse_select_request("t", &[], Some("0-24"), None).unwrap();
        assert_eq!(plan.render().unwrap().0, "SELECT * FROM t LIMIT 25 OFFSET 0");
    }

    #[test]
    fn prefer_count_parsed() {
        let plan =
            parse_select_request("t", &[], None, Some("count=exact, return=representation")).unwrap();
        assert_eq!(plan.count, CountStrategy::Exact);
    }

    #[test]
    fn unsafe_relation_rejected() {
        assert!(matches!(
            parse_select_request("t; DROP", &[], None, None).unwrap_err(),
            ParseError::UnsafeRelation(_)
        ));
    }

    #[test]
    fn unknown_operator_rejected() {
        assert!(matches!(
            parse_select_request("t", &[p("a", "bogus.1")], None, None).unwrap_err(),
            ParseError::UnknownOp(_)
        ));
    }

    #[test]
    fn fts_without_config_parses_to_to_tsquery() {
        let plan = parse_select_request("t", &[p("body", "fts.friend")], None, None).unwrap();
        let (sql, params) = plan.render().unwrap();
        assert_eq!(sql, "SELECT * FROM t WHERE body @@ to_tsquery($1)");
        assert_eq!(params, vec![QueryParam::Text("friend".into())]);
    }

    #[test]
    fn fts_with_config_parses_to_quoted_regconfig() {
        let plan = parse_select_request("t", &[p("body", "fts(english).friend")], None, None)
            .unwrap();
        let (sql, params) = plan.render().unwrap();
        assert_eq!(sql, "SELECT * FROM t WHERE body @@ to_tsquery('english', $1)");
        assert_eq!(params, vec![QueryParam::Text("friend".into())]);
    }

    #[test]
    fn all_four_fts_ops_parse_at_request_level() {
        for (tok, func) in [
            ("fts", "to_tsquery"),
            ("plfts", "plainto_tsquery"),
            ("phfts", "phraseto_tsquery"),
            ("wfts", "websearch_to_tsquery"),
        ] {
            let plan =
                parse_select_request("t", &[p("c", &format!("{tok}(english).q"))], None, None)
                    .unwrap();
            assert_eq!(
                plan.render().unwrap().0,
                format!("SELECT * FROM t WHERE c @@ {func}('english', $1)")
            );
        }
    }

    #[test]
    fn not_prefix_negates_fts() {
        let plan =
            parse_select_request("t", &[p("body", "not.fts(english).q")], None, None).unwrap();
        assert_eq!(
            plan.render().unwrap().0,
            "SELECT * FROM t WHERE NOT (body @@ to_tsquery('english', $1))"
        );
    }

    #[test]
    fn fts_config_rejects_injection_at_parse() {
        let err = parse_select_request("t", &[p("c", "fts(english'); DROP).x")], None, None)
            .unwrap_err();
        assert!(
            matches!(
                err,
                ParseError::Filter(FilterError::Render(RenderError::InvalidFtsConfig(_)))
            ),
            "injection config must be rejected, got {err:?}"
        );
    }

    #[test]
    fn quantifier_path_still_works_for_non_fts() {
        // The generic-suffix reorder must NOT break the `(any)`/`(all)` path.
        let plan = parse_select_request("t", &[p("id", "eq(any).(1,2)")], None, None).unwrap();
        let (sql, params) = plan.render().unwrap();
        assert_eq!(sql, "SELECT * FROM t WHERE id = ANY($1)");
        assert_eq!(
            params,
            vec![QueryParam::TextArray(vec!["1".into(), "2".into()])]
        );
    }

    #[test]
    fn bad_quantifier_payload_on_non_fts_is_rejected() {
        // A non-`(any)`/`(all)` paren payload on a non-FTS op is malformed, not a config.
        let err = parse_select_request("t", &[p("id", "eq(english).1")], None, None).unwrap_err();
        assert!(matches!(err, ParseError::MalformedFilter(_)), "got {err:?}");
    }

    #[test]
    fn fts_inside_logical_group_uses_config() {
        let plan =
            parse_select_request("t", &[p("or", "(body.fts(english).cat,title.fts.dog)")], None, None)
                .unwrap();
        assert_eq!(
            plan.render().unwrap().0,
            "SELECT * FROM t WHERE (body @@ to_tsquery('english', $1) OR title @@ to_tsquery($2))"
        );
    }
}
