//! Condition rendering: [`render_condition`] and its two helpers — the `is`
//! literal validator and the `in`/quantifier list splitter.

use super::{Operator, Quantifier, RenderError};
use crate::fts::{render_fts, FtsConfig};
use crate::param::QueryParam;

/// Render a single condition `<col_ref> <op> <value>` into a SQL fragment plus its
/// bind params, starting placeholder numbering at `next_index`.
///
/// `col_ref` MUST already be validated/quoted by the caller (it is emitted
/// verbatim). `value` is the raw filter value token. `negate` wraps the whole
/// condition in `NOT (...)`. `quantifier` applies `ANY`/`ALL` array semantics.
///
/// `fts_config` carries the optional text-search `regconfig` for the four FTS
/// operators; it MUST be `None` for every non-FTS operator (the parser enforces
/// this) and is ignored there.
///
/// Returns `(sql_fragment, params, next_index_after)`.
///
/// # Errors
/// Returns [`RenderError`] when the `is` operand is not a recognized literal or a
/// quantifier is applied to an operator that does not support it (including any
/// of the four FTS operators, which never accept a quantifier).
pub fn render_condition(
    col_ref: &str,
    op: Operator,
    value: &str,
    negate: bool,
    quantifier: Option<Quantifier>,
    fts_config: Option<&FtsConfig>,
    next_index: usize,
) -> Result<(String, Vec<QueryParam>, usize), RenderError> {
    // Full-text-search operators render bespoke `<col> @@ <fn>([cfg,] $n)` and
    // never accept a quantifier; the paren suffix is always a config.
    if let Some(kind) = op.fts_kind() {
        if quantifier.is_some() {
            return Err(RenderError::QuantifierNotAllowed(kind.token()));
        }
        return Ok(render_fts(
            col_ref, kind, fts_config, value, negate, next_index,
        ));
    }

    let (frag, params, idx) = match op {
        Operator::Is => {
            if quantifier.is_some() {
                return Err(RenderError::QuantifierNotAllowed("is"));
            }
            let literal = parse_is_literal(value)?;
            (format!("{col_ref} IS {literal}"), vec![], next_index)
        }
        Operator::In => {
            if quantifier.is_some() {
                return Err(RenderError::QuantifierNotAllowed("in"));
            }
            // `col IN (a,b,c)` → `col = ANY($n)` with a single text[] bind.
            let items = split_in_list(value);
            (
                format!("{col_ref} = ANY(${next_index})"),
                vec![QueryParam::TextArray(items)],
                next_index + 1,
            )
        }
        scalar => {
            let infix = scalar
                .sql_infix()
                .expect("non In/Is operator always has an infix");
            if let Some(q) = quantifier {
                if !scalar.allows_quantifier() {
                    return Err(RenderError::QuantifierNotAllowed(infix));
                }
                let kw = match q {
                    Quantifier::Any => "ANY",
                    Quantifier::All => "ALL",
                };
                let items = split_in_list(value);
                (
                    format!("{col_ref} {infix} {kw}(${next_index})"),
                    vec![QueryParam::TextArray(items)],
                    next_index + 1,
                )
            } else {
                let param = if scalar.binds_json() {
                    QueryParam::Json(value.to_owned())
                } else {
                    QueryParam::Text(value.to_owned())
                };
                (
                    format!("{col_ref} {infix} ${next_index}"),
                    vec![param],
                    next_index + 1,
                )
            }
        }
    };

    if negate {
        Ok((format!("NOT ({frag})"), params, idx))
    } else {
        Ok((frag, params, idx))
    }
}

/// Validate and normalize an `is` operand to a safe inline SQL literal.
fn parse_is_literal(value: &str) -> Result<&'static str, RenderError> {
    Ok(match value.to_ascii_lowercase().as_str() {
        "null" => "NULL",
        "true" => "TRUE",
        "false" => "FALSE",
        "unknown" => "UNKNOWN",
        _ => return Err(RenderError::InvalidIs(value.to_owned())),
    })
}

/// Split an `in`/quantifier list value into elements.
///
/// PostgREST wraps the list in parentheses (`in.(1,2,3)`); we tolerate both the
/// parenthesized and bare forms. Elements may be double-quoted to protect commas;
/// quotes are stripped and `\"`/`\\` unescaped. Nested parentheses — composite /
/// row-value elements like `((1,2),(3,4))` — are respected via a depth counter, so
/// only top-level commas (depth 0, outside quotes) split the list; the inner tuples
/// survive intact (`["(1,2)", "(3,4)"]`). An empty list yields no elements
/// (an empty `text[]`), which correctly matches no rows.
fn split_in_list(raw: &str) -> Vec<String> {
    let inner = raw
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(raw);
    if inner.is_empty() {
        return vec![];
    }
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut in_quotes = false;
    let mut escaped = false;
    let mut depth = 0u32;
    for c in inner.chars() {
        if escaped {
            buf.push(c);
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            in_quotes = !in_quotes;
        } else if in_quotes {
            buf.push(c);
        } else if c == '(' {
            depth += 1;
            buf.push(c);
        } else if c == ')' {
            depth = depth.saturating_sub(1);
            buf.push(c);
        } else if c == ',' && depth == 0 {
            out.push(std::mem::take(&mut buf));
        } else {
            buf.push(c);
        }
    }
    out.push(buf);
    out
}
