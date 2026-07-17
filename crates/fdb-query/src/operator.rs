//! PostgREST horizontal-filtering operators.
//!
//! The full operator surface: comparison, pattern, membership, null, range, and
//! containment operators, plus the `not.` negation prefix and the `any()`/`all()`
//! modifiers. Rendering always binds user values as parameters; only the operator
//! keyword (a fixed string) and the already-validated column reference reach SQL
//! directly.

use crate::fts::{render_fts, FtsConfig, FtsKind};
use crate::param::QueryParam;

/// A PostgREST filter operator.
///
/// Token → operator mapping follows the PostgREST "Operators" reference. Range
/// operators (`sl`, `sr`, `nxr`, `nxl`, `adj`, `ov`) target range/array columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Operator {
    /// `eq` → `=`
    Eq,
    /// `neq` → `<>`
    Neq,
    /// `gt` → `>`
    Gt,
    /// `gte` → `>=`
    Gte,
    /// `lt` → `<`
    Lt,
    /// `lte` → `<=`
    Lte,
    /// `like` → `LIKE`
    Like,
    /// `ilike` → `ILIKE`
    Ilike,
    /// `match` → `~` (POSIX regex)
    Match,
    /// `imatch` → `~*` (case-insensitive POSIX regex)
    Imatch,
    /// `in` → `= ANY(...)`
    In,
    /// `is` → `IS` (null / true / false / unknown)
    Is,
    /// `isdistinct` → `IS DISTINCT FROM`
    IsDistinct,
    /// `cs` → `@>` (contains)
    Cs,
    /// `cd` → `<@` (contained by)
    Cd,
    /// `ov` → `&&` (overlap)
    Ov,
    /// `sl` → `<<` (strictly left of)
    Sl,
    /// `sr` → `>>` (strictly right of)
    Sr,
    /// `nxr` → `&<` (does not extend to the right of)
    Nxr,
    /// `nxl` → `&>` (does not extend to the left of)
    Nxl,
    /// `adj` → `-|-` (adjacent)
    Adj,
    /// `fts` → `@@ to_tsquery(...)` (full-text search)
    Fts,
    /// `plfts` → `@@ plainto_tsquery(...)` (plain full-text search)
    Plfts,
    /// `phfts` → `@@ phraseto_tsquery(...)` (phrase full-text search)
    Phfts,
    /// `wfts` → `@@ websearch_to_tsquery(...)` (web-search full-text search)
    Wfts,
}

/// The `any`/`all` modifier applied to a scalar comparison operator, e.g.
/// `?id=eq(any).{1,2,3}` → `id = ANY('{1,2,3}')`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Quantifier {
    /// `= ANY(array)` — true if the comparison holds for any element.
    Any,
    /// `= ALL(array)` — true only if the comparison holds for every element.
    All,
}

impl Operator {
    /// Parse the operator keyword (the token before the first `.` in a filter,
    /// after any `not.` prefix and any `(any)`/`(all)` modifier have been stripped).
    #[must_use]
    pub fn parse(token: &str) -> Option<Self> {
        Some(match token {
            "eq" => Self::Eq,
            "neq" => Self::Neq,
            "gt" => Self::Gt,
            "gte" => Self::Gte,
            "lt" => Self::Lt,
            "lte" => Self::Lte,
            "like" => Self::Like,
            "ilike" => Self::Ilike,
            "match" => Self::Match,
            "imatch" => Self::Imatch,
            "in" => Self::In,
            "is" => Self::Is,
            "isdistinct" => Self::IsDistinct,
            "cs" => Self::Cs,
            "cd" => Self::Cd,
            "ov" => Self::Ov,
            "sl" => Self::Sl,
            "sr" => Self::Sr,
            "nxr" => Self::Nxr,
            "nxl" => Self::Nxl,
            "adj" => Self::Adj,
            "fts" => Self::Fts,
            "plfts" => Self::Plfts,
            "phfts" => Self::Phfts,
            "wfts" => Self::Wfts,
            _ => return None,
        })
    }

    /// The SQL infix keyword for operators that render as `col <op> $n`.
    ///
    /// Returns `None` for operators with bespoke rendering (`In`, `Is`), which the
    /// renderer handles separately.
    #[must_use]
    pub fn sql_infix(self) -> Option<&'static str> {
        Some(match self {
            Self::Eq => "=",
            Self::Neq => "<>",
            Self::Gt => ">",
            Self::Gte => ">=",
            Self::Lt => "<",
            Self::Lte => "<=",
            Self::Like => "LIKE",
            Self::Ilike => "ILIKE",
            Self::Match => "~",
            Self::Imatch => "~*",
            Self::IsDistinct => "IS DISTINCT FROM",
            Self::Cs => "@>",
            Self::Cd => "<@",
            Self::Ov => "&&",
            Self::Sl => "<<",
            Self::Sr => ">>",
            Self::Nxr => "&<",
            Self::Nxl => "&>",
            Self::Adj => "-|-",
            Self::In | Self::Is | Self::Fts | Self::Plfts | Self::Phfts | Self::Wfts => {
                return None;
            }
        })
    }

    /// The full-text-search kind for this operator, if any.
    ///
    /// Returns `Some(..)` only for the four FTS operators; `None` otherwise. Used
    /// by [`render_condition`] to branch into [`render_fts`] and by the parser to
    /// decide the paren suffix is a text-search config rather than a quantifier.
    #[must_use]
    pub fn fts_kind(self) -> Option<FtsKind> {
        Some(match self {
            Self::Fts => FtsKind::Fts,
            Self::Plfts => FtsKind::Plfts,
            Self::Phfts => FtsKind::Phfts,
            Self::Wfts => FtsKind::Wfts,
            _ => return None,
        })
    }

    /// Whether the `any`/`all` quantifier modifier is valid for this operator.
    /// PostgREST allows it on the scalar comparison and pattern operators.
    #[must_use]
    pub fn allows_quantifier(self) -> bool {
        matches!(
            self,
            Self::Eq
                | Self::Neq
                | Self::Gt
                | Self::Gte
                | Self::Lt
                | Self::Lte
                | Self::Like
                | Self::Ilike
                | Self::Match
                | Self::Imatch
        )
    }

    /// Whether this operator's bound value should be encoded as a `jsonb` literal
    /// rather than plain text. Containment operators compare JSON/array structure.
    #[must_use]
    pub fn binds_json(self) -> bool {
        matches!(self, Self::Cs | Self::Cd)
    }
}

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
/// `cast` is an optional, already-validated Postgres base type (e.g. `int4`).
/// When present, it is appended to the bound placeholder (`$n::int4`, or
/// `$n::int4[]` for an array bind) so a driver that declares the bound value's
/// type explicitly as text (`sqlx`) still resolves against a non-text column.
/// Ignored for containment operators (already bind `jsonb`) and `is` (no bind).
///
/// Returns `(sql_fragment, params, next_index_after)`.
///
/// # Errors
/// Returns [`RenderError`] when the `is` operand is not a recognized literal or a
/// quantifier is applied to an operator that does not support it (including any
/// of the four FTS operators, which never accept a quantifier).
#[allow(clippy::too_many_arguments)] // each param is an independently-meaningful render input; grouping into a struct would not clarify this low-level primitive
pub fn render_condition(
    col_ref: &str,
    op: Operator,
    value: &str,
    negate: bool,
    quantifier: Option<Quantifier>,
    fts_config: Option<&FtsConfig>,
    cast: Option<&str>,
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
            let ph = array_placeholder(next_index, cast);
            (
                format!("{col_ref} = ANY({ph})"),
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
                let ph = array_placeholder(next_index, cast);
                (
                    format!("{col_ref} {infix} {kw}({ph})"),
                    vec![QueryParam::TextArray(items)],
                    next_index + 1,
                )
            } else {
                let param = if scalar.binds_json() {
                    QueryParam::Json(value.to_owned())
                } else {
                    QueryParam::Text(value.to_owned())
                };
                // Containment operators already bind `jsonb`; never cast them.
                let ph = if scalar.binds_json() {
                    format!("${next_index}")
                } else {
                    scalar_placeholder(next_index, cast)
                };
                (
                    format!("{col_ref} {infix} {ph}"),
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

/// A scalar placeholder, cast to `cast` when present: `$n` or `$n::int4`.
fn scalar_placeholder(idx: usize, cast: Option<&str>) -> String {
    match cast {
        Some(t) => format!("${idx}::{t}"),
        None => format!("${idx}"),
    }
}

/// An array-bind placeholder, cast to `cast[]` when present: `$n` or `$n::int4[]`.
fn array_placeholder(idx: usize, cast: Option<&str>) -> String {
    match cast {
        Some(t) => format!("${idx}::{t}[]"),
        None => format!("${idx}"),
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

/// Errors produced while rendering a condition.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RenderError {
    /// The `is` operand was not one of null/true/false/unknown.
    #[error("invalid `is` value: {0} (expected null/true/false/unknown)")]
    InvalidIs(String),
    /// An `any`/`all` quantifier was applied to an operator that does not accept it.
    #[error("operator `{0}` does not accept an any/all quantifier")]
    QuantifierNotAllowed(&'static str),
    /// A full-text-search config name failed identifier validation.
    #[error("invalid text-search config: {0}")]
    InvalidFtsConfig(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_operator_tokens() {
        for tok in [
            "eq",
            "neq",
            "gt",
            "gte",
            "lt",
            "lte",
            "like",
            "ilike",
            "match",
            "imatch",
            "in",
            "is",
            "isdistinct",
            "cs",
            "cd",
            "ov",
            "sl",
            "sr",
            "nxr",
            "nxl",
            "adj",
        ] {
            assert!(Operator::parse(tok).is_some(), "token {tok} should parse");
        }
        assert!(Operator::parse("bogus").is_none());
    }

    fn render(col: &str, op: Operator, val: &str) -> (String, Vec<QueryParam>) {
        let (sql, params, _) =
            render_condition(col, op, val, false, None, None, None, 1).expect("render");
        (sql, params)
    }

    fn render_cast(col: &str, op: Operator, val: &str, cast: &str) -> (String, Vec<QueryParam>) {
        let (sql, params, _) =
            render_condition(col, op, val, false, None, None, Some(cast), 1).expect("render");
        (sql, params)
    }

    #[test]
    fn scalar_operators_render_parameterized() {
        assert_eq!(render("c", Operator::Eq, "1").0, "c = $1");
        assert_eq!(render("c", Operator::Neq, "1").0, "c <> $1");
        assert_eq!(render("c", Operator::Gt, "1").0, "c > $1");
        assert_eq!(render("c", Operator::Gte, "1").0, "c >= $1");
        assert_eq!(render("c", Operator::Lt, "1").0, "c < $1");
        assert_eq!(render("c", Operator::Lte, "1").0, "c <= $1");
        assert_eq!(render("c", Operator::Like, "a%").0, "c LIKE $1");
        assert_eq!(render("c", Operator::Ilike, "a%").0, "c ILIKE $1");
        assert_eq!(render("c", Operator::Match, "^a").0, "c ~ $1");
        assert_eq!(render("c", Operator::Imatch, "^a").0, "c ~* $1");
        assert_eq!(
            render("c", Operator::IsDistinct, "1").0,
            "c IS DISTINCT FROM $1"
        );
    }

    #[test]
    fn range_operators_render_correct_symbols() {
        assert_eq!(render("r", Operator::Ov, "[1,2]").0, "r && $1");
        assert_eq!(render("r", Operator::Sl, "[1,2]").0, "r << $1");
        assert_eq!(render("r", Operator::Sr, "[1,2]").0, "r >> $1");
        assert_eq!(render("r", Operator::Nxr, "[1,2]").0, "r &< $1");
        assert_eq!(render("r", Operator::Nxl, "[1,2]").0, "r &> $1");
        assert_eq!(render("r", Operator::Adj, "[1,2]").0, "r -|- $1");
    }

    #[test]
    fn containment_operators_bind_json() {
        let (sql, params) = render("tags", Operator::Cs, r#"{"a":1}"#);
        assert_eq!(sql, "tags @> $1");
        assert!(matches!(params[0], QueryParam::Json(_)));
        assert_eq!(render("tags", Operator::Cd, "{1,2}").0, "tags <@ $1");
    }

    #[test]
    fn in_operator_uses_any_with_text_array() {
        let (sql, params) = render("id", Operator::In, "(1,2,3)");
        assert_eq!(sql, "id = ANY($1)");
        assert_eq!(
            params,
            vec![QueryParam::TextArray(vec![
                "1".into(),
                "2".into(),
                "3".into()
            ])]
        );
    }

    #[test]
    fn in_list_tolerates_bare_and_quoted_forms() {
        // bare (no parens)
        let (_, p) = render("id", Operator::In, "1,2");
        assert_eq!(p, vec![QueryParam::TextArray(vec!["1".into(), "2".into()])]);
        // quoted element protecting a comma
        let (_, p) = render("name", Operator::In, r#"("a,b",c)"#);
        assert_eq!(
            p,
            vec![QueryParam::TextArray(vec!["a,b".into(), "c".into()])]
        );
    }

    #[test]
    fn empty_in_list_yields_empty_array() {
        let (sql, params) = render("id", Operator::In, "()");
        assert_eq!(sql, "id = ANY($1)");
        assert_eq!(params, vec![QueryParam::TextArray(vec![])]);
    }

    #[test]
    fn is_operator_inlines_validated_literal_no_bind() {
        for (val, expect) in [
            ("null", "c IS NULL"),
            ("true", "c IS TRUE"),
            ("false", "c IS FALSE"),
            ("UNKNOWN", "c IS UNKNOWN"),
        ] {
            let (sql, params) = render("c", Operator::Is, val);
            assert_eq!(sql, expect);
            assert!(params.is_empty());
        }
    }

    #[test]
    fn is_operator_rejects_bad_operand() {
        let err =
            render_condition("c", Operator::Is, "maybe", false, None, None, None, 1).unwrap_err();
        assert!(matches!(err, RenderError::InvalidIs(_)));
    }

    #[test]
    fn negation_wraps_condition() {
        let (sql, _, _) =
            render_condition("c", Operator::Eq, "1", true, None, None, None, 1).expect("render");
        assert_eq!(sql, "NOT (c = $1)");
    }

    #[test]
    fn quantifier_any_all_render() {
        let (sql, params, _) = render_condition(
            "c",
            Operator::Eq,
            "(1,2)",
            false,
            Some(Quantifier::Any),
            None,
            None,
            1,
        )
        .expect("render");
        assert_eq!(sql, "c = ANY($1)");
        assert_eq!(
            params,
            vec![QueryParam::TextArray(vec!["1".into(), "2".into()])]
        );

        let (sql, _, _) = render_condition(
            "c",
            Operator::Like,
            "(a%,b%)",
            false,
            Some(Quantifier::All),
            None,
            None,
            1,
        )
        .expect("render");
        assert_eq!(sql, "c LIKE ALL($1)");
    }

    #[test]
    fn quantifier_rejected_on_unsupported_operators() {
        for op in [Operator::In, Operator::Is, Operator::Cs, Operator::Ov] {
            let err = render_condition("c", op, "x", false, Some(Quantifier::Any), None, None, 1)
                .unwrap_err();
            assert!(
                matches!(err, RenderError::QuantifierNotAllowed(_)),
                "op {op:?} should reject quantifier"
            );
        }
    }

    #[test]
    fn index_advances_by_bind_count() {
        let (_, _, next) =
            render_condition("c", Operator::Eq, "1", false, None, None, None, 5).expect("render");
        assert_eq!(next, 6, "one bind consumed");
        let (_, _, next) = render_condition("c", Operator::Is, "null", false, None, None, None, 5)
            .expect("render");
        assert_eq!(next, 5, "is binds nothing");
    }

    #[test]
    fn all_four_fts_tokens_parse_and_map_to_kinds() {
        for (tok, kind) in [
            ("fts", FtsKind::Fts),
            ("plfts", FtsKind::Plfts),
            ("phfts", FtsKind::Phfts),
            ("wfts", FtsKind::Wfts),
        ] {
            let op = Operator::parse(tok).unwrap_or_else(|| panic!("token {tok} should parse"));
            assert_eq!(op.fts_kind(), Some(kind), "token {tok} maps to its kind");
        }
    }

    #[test]
    fn non_fts_operators_have_no_fts_kind() {
        for op in [Operator::Eq, Operator::In, Operator::Is, Operator::Cs] {
            assert_eq!(op.fts_kind(), None, "op {op:?} is not an FTS op");
        }
    }

    #[test]
    fn fts_operators_have_no_infix() {
        for op in [
            Operator::Fts,
            Operator::Plfts,
            Operator::Phfts,
            Operator::Wfts,
        ] {
            assert_eq!(
                op.sql_infix(),
                None,
                "FTS op {op:?} renders bespoke, not infix"
            );
        }
    }

    #[test]
    fn render_condition_dispatches_fts_without_config() {
        let (sql, params, next) =
            render_condition("c", Operator::Fts, "cat & dog", false, None, None, None, 1)
                .expect("render");
        assert_eq!(sql, "c @@ to_tsquery($1)");
        assert_eq!(params, vec![QueryParam::Text("cat & dog".into())]);
        assert_eq!(next, 2);
    }

    #[test]
    fn render_condition_dispatches_fts_with_config() {
        let cfg = FtsConfig::parse("english").unwrap();
        let (sql, params, _) =
            render_condition("c", Operator::Fts, "cat", false, None, Some(&cfg), None, 1)
                .expect("render");
        assert_eq!(sql, "c @@ to_tsquery('english', $1)");
        assert_eq!(params, vec![QueryParam::Text("cat".into())]);
    }

    #[test]
    fn quantifier_rejected_on_all_fts_ops() {
        for op in [
            Operator::Fts,
            Operator::Plfts,
            Operator::Phfts,
            Operator::Wfts,
        ] {
            let err = render_condition("c", op, "q", false, Some(Quantifier::Any), None, None, 1)
                .unwrap_err();
            assert!(
                matches!(err, RenderError::QuantifierNotAllowed(_)),
                "FTS op {op:?} must reject quantifier"
            );
        }
    }

    #[test]
    fn cast_appends_to_scalar_placeholder() {
        assert_eq!(
            render_cast("id", Operator::Eq, "1", "int4").0,
            "id = $1::int4"
        );
        assert_eq!(
            render_cast("id", Operator::Gte, "1", "int8").0,
            "id >= $1::int8"
        );
    }

    #[test]
    fn cast_appends_array_suffix_for_in_and_quantifiers() {
        let (sql, _, _) = render_condition(
            "id",
            Operator::In,
            "(1,2)",
            false,
            None,
            None,
            Some("int4"),
            1,
        )
        .expect("render");
        assert_eq!(sql, "id = ANY($1::int4[])");

        let (sql, _, _) = render_condition(
            "id",
            Operator::Eq,
            "(1,2)",
            false,
            Some(Quantifier::Any),
            None,
            Some("int4"),
            1,
        )
        .expect("render");
        assert_eq!(sql, "id = ANY($1::int4[])");
    }

    #[test]
    fn cast_is_ignored_for_containment_and_is() {
        // Containment operators already bind jsonb; casting would be wrong.
        assert_eq!(
            render_cast("tags", Operator::Cs, r#"{"a":1}"#, "int4").0,
            "tags @> $1"
        );
        // `is` never binds a param, so there is nothing to cast.
        let (sql, params, _) = render_condition(
            "c",
            Operator::Is,
            "null",
            false,
            None,
            None,
            Some("int4"),
            1,
        )
        .expect("render");
        assert_eq!(sql, "c IS NULL");
        assert!(params.is_empty());
    }

    #[test]
    fn render_condition_negates_fts() {
        let (sql, _, _) =
            render_condition("c", Operator::Fts, "q", true, None, None, None, 1).expect("render");
        assert_eq!(sql, "NOT (c @@ to_tsquery($1))");
    }
}
