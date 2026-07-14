//! Filter/logical-group parsing helpers used by [`super::parse_select_request`].

use super::ParseError;
use crate::clause::{split_top_level_commas, CountStrategy};
use crate::filter::{FilterError, FilterTree};
use crate::fts::FtsConfig;
use crate::operator::{Operator, Quantifier, RenderError};

/// Parse a single `column=value` filter into a leaf, honoring `not.` and
/// `op(any)`/`op(all)` modifiers. Value form: `[not.]op[(any|all)].operand`.
pub(super) fn parse_leaf(column: &str, raw: &str) -> Result<FilterTree, ParseError> {
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
        let cfg = payload.map(FtsConfig::parse).transpose().map_err(|e| {
            ParseError::Filter(FilterError::Render(RenderError::InvalidFtsConfig(
                e.to_string(),
            )))
        })?;
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
pub(super) fn parse_logical_group(kind: &str, raw: &str) -> Result<FilterTree, ParseError> {
    let inner = raw
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| ParseError::MalformedGroup(raw.to_owned()))?;

    let members = split_top_level_commas(inner);
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
pub(super) fn parse_count_from_prefer(prefer: &str) -> CountStrategy {
    for part in prefer.split([',', ' ']) {
        if let Some(v) = part.trim().strip_prefix("count=") {
            return CountStrategy::parse(v);
        }
    }
    CountStrategy::None
}
