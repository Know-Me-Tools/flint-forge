use super::*;
use crate::filter::FilterError;
use crate::operator::RenderError;

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
    assert_eq!(
        plan.render().unwrap().0,
        "SELECT * FROM t WHERE NOT (a = $1)"
    );
}

#[test]
fn quantifier_suffix_on_leaf() {
    let plan = parse_select_request("t", &[p("id", "eq(any).(1,2,3)")], None, None).unwrap();
    let (sql, params) = plan.render().unwrap();
    assert_eq!(sql, "SELECT * FROM t WHERE id = ANY($1)");
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
fn logical_or_group() {
    let plan = parse_select_request("t", &[p("or", "(a.eq.1,b.eq.2)")], None, None).unwrap();
    assert_eq!(
        plan.render().unwrap().0,
        "SELECT * FROM t WHERE (a = $1 OR b = $2)"
    );
}

#[test]
fn nested_and_or_group() {
    let plan =
        parse_select_request("t", &[p("and", "(a.gt.1,or(b.eq.2,c.eq.3))")], None, None).unwrap();
    assert_eq!(
        plan.render().unwrap().0,
        "SELECT * FROM t WHERE (a > $1 AND (b = $2 OR c = $3))"
    );
}

#[test]
fn not_and_group_negates() {
    let plan = parse_select_request("t", &[p("not.and", "(a.gte.0,a.lte.9)")], None, None).unwrap();
    assert_eq!(
        plan.render().unwrap().0,
        "SELECT * FROM t WHERE NOT ((a >= $1 AND a <= $2))"
    );
}

#[test]
fn range_header_overrides_limit() {
    let plan = parse_select_request("t", &[], Some("0-24"), None).unwrap();
    assert_eq!(
        plan.render().unwrap().0,
        "SELECT * FROM t LIMIT 25 OFFSET 0"
    );
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
    let plan = parse_select_request("t", &[p("body", "fts(english).friend")], None, None).unwrap();
    let (sql, params) = plan.render().unwrap();
    assert_eq!(
        sql,
        "SELECT * FROM t WHERE body @@ to_tsquery('english', $1)"
    );
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
            parse_select_request("t", &[p("c", &format!("{tok}(english).q"))], None, None).unwrap();
        assert_eq!(
            plan.render().unwrap().0,
            format!("SELECT * FROM t WHERE c @@ {func}('english', $1)")
        );
    }
}

#[test]
fn not_prefix_negates_fts() {
    let plan = parse_select_request("t", &[p("body", "not.fts(english).q")], None, None).unwrap();
    assert_eq!(
        plan.render().unwrap().0,
        "SELECT * FROM t WHERE NOT (body @@ to_tsquery('english', $1))"
    );
}

#[test]
fn fts_config_rejects_injection_at_parse() {
    let err =
        parse_select_request("t", &[p("c", "fts(english'); DROP).x")], None, None).unwrap_err();
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
    let plan = parse_select_request(
        "t",
        &[p("or", "(body.fts(english).cat,title.fts.dog)")],
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        plan.render().unwrap().0,
        "SELECT * FROM t WHERE (body @@ to_tsquery('english', $1) OR title @@ to_tsquery($2))"
    );
}
