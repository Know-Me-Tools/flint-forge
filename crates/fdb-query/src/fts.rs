//! Full-text-search operator support for the PostgREST filter surface.
//!
//! PostgREST exposes four full-text-search operators — `fts`, `plfts`, `phfts`,
//! and `wfts` — which map onto Postgres' four `tsquery` constructors. Every one
//! renders as `<col> @@ <fn>([config,] $n)` where:
//!
//! * `@@` and the four `*_tsquery` function names are compile-time constants;
//! * the tsquery **text** is ALWAYS a bound parameter ([`QueryParam::Text`]) —
//!   nothing user-supplied is ever interpolated;
//! * the optional language `config` (e.g. `english`) is a validated identifier
//!   emitted as a single-quoted `regconfig` literal.
//!
//! The config rides in the operator token's paren suffix —
//! `col=fts(english).query` — the exact lexical slot the parser already strips
//! for the `(any)`/`(all)` quantifier. FTS operators reject quantifiers, so the
//! suffix on an FTS op is unambiguously a config.

use crate::ident::{IdentError, validate_identifier};
use crate::param::QueryParam;

/// Which `tsquery` constructor a full-text-search operator uses.
///
/// Each variant maps 1:1 to one of the four PostgREST FTS operators and to one
/// Postgres text-search function. [`Copy`] so it threads freely through the
/// render path alongside the (also `Copy`) [`crate::operator::Operator`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FtsKind {
    /// `fts` → `to_tsquery(...)` — raw tsquery syntax (`&`, `|`, `!`, `<->`).
    Fts,
    /// `plfts` → `plainto_tsquery(...)` — plain text, terms AND-ed.
    Plfts,
    /// `phfts` → `phraseto_tsquery(...)` — plain text as an ordered phrase.
    Phfts,
    /// `wfts` → `websearch_to_tsquery(...)` — web-search syntax (quotes, `OR`, `-`).
    Wfts,
}

impl FtsKind {
    /// The Postgres text-search function this kind renders to. A compile-time
    /// constant `&'static str` — never influenced by user input.
    #[must_use]
    pub fn sql_fn(self) -> &'static str {
        match self {
            Self::Fts => "to_tsquery",
            Self::Plfts => "plainto_tsquery",
            Self::Phfts => "phraseto_tsquery",
            Self::Wfts => "websearch_to_tsquery",
        }
    }

    /// The PostgREST operator token for this kind (for round-tripping / tests).
    #[must_use]
    pub fn token(self) -> &'static str {
        match self {
            Self::Fts => "fts",
            Self::Plfts => "plfts",
            Self::Phfts => "phfts",
            Self::Wfts => "wfts",
        }
    }
}

/// A validated text-search `regconfig` name, e.g. `english`.
///
/// The ONLY constructor is [`FtsConfig::parse`], which delegates to
/// [`validate_identifier`]; a config can therefore never contain quotes,
/// parentheses, or any injection payload. [`FtsConfig::to_sql_literal`] emits a
/// single-quoted `regconfig` literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FtsConfig(String);

impl FtsConfig {
    /// Parse and validate a text-search config name.
    ///
    /// # Errors
    /// Returns [`IdentError`] when the name fails the segmented
    /// alphanumeric/underscore identifier check (empty, contains spaces,
    /// quotes, parens, or other unsafe characters).
    pub fn parse(s: &str) -> Result<Self, IdentError> {
        let valid = validate_identifier(s)?;
        Ok(Self(valid.to_owned()))
    }

    /// Emit the config as a single-quoted SQL `regconfig` literal, e.g.
    /// `'english'`. Embedded single quotes are doubled defensively, though
    /// [`validate_identifier`] already forbids them.
    #[must_use]
    pub fn to_sql_literal(&self) -> String {
        let escaped = self.0.replace('\'', "''");
        format!("'{escaped}'")
    }

    /// The validated config name, without quoting.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Render a full-text-search condition `<col_ref> @@ <fn>([config,] $n)`.
///
/// `col_ref` MUST already be validated/quoted by the caller (emitted verbatim).
/// `value` is the raw tsquery text, bound as a single [`QueryParam::Text`] — the
/// tsquery constructors do their own safe parsing, so no escaping of `&|!():*`
/// (or of websearch syntax) is required here. `cfg`, when present, is emitted as
/// a single-quoted `regconfig` literal before the placeholder. `negate` wraps
/// the whole condition in `NOT (...)`.
///
/// Returns `(sql_fragment, params, next_index_after)`; exactly one param is
/// bound, so `next_index` always advances by one regardless of config presence.
#[must_use]
pub fn render_fts(
    col_ref: &str,
    kind: FtsKind,
    cfg: Option<&FtsConfig>,
    value: &str,
    negate: bool,
    next_index: usize,
) -> (String, Vec<QueryParam>, usize) {
    let func = kind.sql_fn();
    let frag = match cfg {
        Some(c) => format!("{col_ref} @@ {func}({}, ${next_index})", c.to_sql_literal()),
        None => format!("{col_ref} @@ {func}(${next_index})"),
    };
    let params = vec![QueryParam::Text(value.to_owned())];
    let idx = next_index + 1;
    if negate {
        (format!("NOT ({frag})"), params, idx)
    } else {
        (frag, params, idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_sql_fn_names_are_fixed() {
        assert_eq!(FtsKind::Fts.sql_fn(), "to_tsquery");
        assert_eq!(FtsKind::Plfts.sql_fn(), "plainto_tsquery");
        assert_eq!(FtsKind::Phfts.sql_fn(), "phraseto_tsquery");
        assert_eq!(FtsKind::Wfts.sql_fn(), "websearch_to_tsquery");
    }

    #[test]
    fn kind_tokens_round_trip() {
        assert_eq!(FtsKind::Fts.token(), "fts");
        assert_eq!(FtsKind::Plfts.token(), "plfts");
        assert_eq!(FtsKind::Phfts.token(), "phfts");
        assert_eq!(FtsKind::Wfts.token(), "wfts");
    }

    #[test]
    fn config_parse_validates_and_quotes() {
        let cfg = FtsConfig::parse("english").expect("valid config");
        assert_eq!(cfg.as_str(), "english");
        assert_eq!(cfg.to_sql_literal(), "'english'");
    }

    #[test]
    fn config_rejects_injection() {
        let err = FtsConfig::parse("english'); DROP").unwrap_err();
        assert!(matches!(err, IdentError::Unsafe(_)));
    }

    #[test]
    fn config_rejects_empty_and_spaces() {
        assert!(FtsConfig::parse("").is_err());
        assert!(FtsConfig::parse("a b").is_err());
    }

    #[test]
    fn config_rejects_parens() {
        assert!(FtsConfig::parse("eng(lish)").is_err());
    }

    #[test]
    fn fts_renders_to_tsquery_bound() {
        let (sql, params, next) = render_fts("c", FtsKind::Fts, None, "cat & dog", false, 1);
        assert_eq!(sql, "c @@ to_tsquery($1)");
        assert_eq!(params, vec![QueryParam::Text("cat & dog".into())]);
        assert_eq!(next, 2);
    }

    #[test]
    fn fts_with_config_emits_quoted_regconfig() {
        let cfg = FtsConfig::parse("english").unwrap();
        let (sql, params, _) = render_fts("c", FtsKind::Fts, Some(&cfg), "cat", false, 1);
        assert_eq!(sql, "c @@ to_tsquery('english', $1)");
        assert_eq!(params, vec![QueryParam::Text("cat".into())]);
    }

    #[test]
    fn plfts_maps_to_plainto() {
        let (sql, _, _) = render_fts("c", FtsKind::Plfts, None, "cat dog", false, 1);
        assert_eq!(sql, "c @@ plainto_tsquery($1)");
    }

    #[test]
    fn phfts_maps_to_phraseto() {
        let cfg = FtsConfig::parse("english").unwrap();
        let (sql, _, _) = render_fts("c", FtsKind::Phfts, Some(&cfg), "the cat", false, 1);
        assert_eq!(sql, "c @@ phraseto_tsquery('english', $1)");
    }

    #[test]
    fn wfts_maps_to_websearch() {
        let (sql, params, _) =
            render_fts("c", FtsKind::Wfts, None, "\"quoted phrase\" OR term", false, 1);
        assert_eq!(sql, "c @@ websearch_to_tsquery($1)");
        assert_eq!(
            params,
            vec![QueryParam::Text("\"quoted phrase\" OR term".into())]
        );
    }

    #[test]
    fn fts_query_text_is_never_interpolated() {
        let payload = "x'); DROP TABLE t;--";
        let (sql, params, _) = render_fts("c", FtsKind::Fts, None, payload, false, 1);
        assert_eq!(sql, "c @@ to_tsquery($1)");
        assert!(
            !sql.contains("DROP"),
            "user value must not appear in the SQL fragment"
        );
        assert_eq!(params, vec![QueryParam::Text(payload.into())]);
    }

    #[test]
    fn negation_wraps_fts() {
        let (sql, _, _) = render_fts("c", FtsKind::Fts, None, "query", true, 1);
        assert_eq!(sql, "NOT (c @@ to_tsquery($1))");
    }

    #[test]
    fn negation_wraps_fts_with_config() {
        let cfg = FtsConfig::parse("english").unwrap();
        let (sql, _, _) = render_fts("c", FtsKind::Fts, Some(&cfg), "query", true, 1);
        assert_eq!(sql, "NOT (c @@ to_tsquery('english', $1))");
    }

    #[test]
    fn no_config_has_no_leading_comma() {
        let (sql, _, _) = render_fts("c", FtsKind::Fts, None, "q", false, 1);
        assert!(!sql.contains("(, "), "single-arg form must not have a leading comma");
        assert_eq!(sql, "c @@ to_tsquery($1)");
    }

    #[test]
    fn index_advances_by_one_regardless_of_config() {
        let (_, _, next_no_cfg) = render_fts("c", FtsKind::Fts, None, "q", false, 7);
        assert_eq!(next_no_cfg, 8);
        let cfg = FtsConfig::parse("english").unwrap();
        let (_, _, next_cfg) = render_fts("c", FtsKind::Fts, Some(&cfg), "q", false, 7);
        assert_eq!(next_cfg, 8);
    }
}
