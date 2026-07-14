use super::*;
use crate::fts::{FtsConfig, FtsKind};
use crate::param::QueryParam;

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
    let (sql, params, _) = render_condition(col, op, val, false, None, None, 1).expect("render");
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
    let err = render_condition("c", Operator::Is, "maybe", false, None, None, 1).unwrap_err();
    assert!(matches!(err, RenderError::InvalidIs(_)));
}

#[test]
fn negation_wraps_condition() {
    let (sql, _, _) =
        render_condition("c", Operator::Eq, "1", true, None, None, 1).expect("render");
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
        1,
    )
    .expect("render");
    assert_eq!(sql, "c LIKE ALL($1)");
}

#[test]
fn quantifier_rejected_on_unsupported_operators() {
    for op in [Operator::In, Operator::Is, Operator::Cs, Operator::Ov] {
        let err =
            render_condition("c", op, "x", false, Some(Quantifier::Any), None, 1).unwrap_err();
        assert!(
            matches!(err, RenderError::QuantifierNotAllowed(_)),
            "op {op:?} should reject quantifier"
        );
    }
}

#[test]
fn index_advances_by_bind_count() {
    let (_, _, next) =
        render_condition("c", Operator::Eq, "1", false, None, None, 5).expect("render");
    assert_eq!(next, 6, "one bind consumed");
    let (_, _, next) =
        render_condition("c", Operator::Is, "null", false, None, None, 5).expect("render");
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
        render_condition("c", Operator::Fts, "cat & dog", false, None, None, 1).expect("render");
    assert_eq!(sql, "c @@ to_tsquery($1)");
    assert_eq!(params, vec![QueryParam::Text("cat & dog".into())]);
    assert_eq!(next, 2);
}

#[test]
fn render_condition_dispatches_fts_with_config() {
    let cfg = FtsConfig::parse("english").unwrap();
    let (sql, params, _) =
        render_condition("c", Operator::Fts, "cat", false, None, Some(&cfg), 1).expect("render");
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
        let err =
            render_condition("c", op, "q", false, Some(Quantifier::Any), None, 1).unwrap_err();
        assert!(
            matches!(err, RenderError::QuantifierNotAllowed(_)),
            "FTS op {op:?} must reject quantifier"
        );
    }
}

#[test]
fn render_condition_negates_fts() {
    let (sql, _, _) =
        render_condition("c", Operator::Fts, "q", true, None, None, 1).expect("render");
    assert_eq!(sql, "NOT (c @@ to_tsquery($1))");
}
