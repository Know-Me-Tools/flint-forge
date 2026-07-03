//! PostgREST edge-case regression + fix suite for `fdb-query`.
//!
//! This integration test locks in the behavior of eleven tricky PostgREST corner
//! cases as they flow through the *public* API of the pure translator
//! (`parse_select_request`, `render_condition`, `Operator`, `Quantifier`,
//! `QueryParam`, `Limits`, `Order`, `Select`). It exercises no DB and no async.
//!
//! Nine of the cases were already correct before this suite existed and are pinned
//! here so future refactors cannot silently regress them. Two were genuine parser
//! defects and are proven fixed:
//!
//!   (A) Composite / row-value in-lists like `id=in.((1,2),(3,4))` were mis-split
//!       by `split_in_list` (quote-aware but not paren-depth-aware), yielding four
//!       bogus elements `["(1","2)","(3","4)"]`. Now the inner tuples survive.
//!   (B) A logical-group member whose column name merely *starts* with the literal
//!       text `and`/`or` (e.g. `android.eq.1`, `origin.eq.x`) was mis-detected as a
//!       nested group by `strip_prefix("and")`/`strip_prefix("or")`, producing a
//!       spurious 400. Now only `and(` / `or(` (keyword immediately followed by an
//!       open paren) dispatch to a nested group.
//!
//! Neither defect was an injection hole: every user value is still a bound `$n`
//! parameter and every identifier still passes `validate_identifier`. These are
//! wrong-SQL / wrong-error bugs, and the tests assert the exact `(sql, params)`.

use fdb_query::{
    parse_select_request, render_condition, Limits, Operator, Order, ParseError, Quantifier,
    QueryParam, RenderError, Select,
};

/// Build a `(key, value)` query-param pair.
fn p(k: &str, v: &str) -> (String, String) {
    (k.to_owned(), v.to_owned())
}

/// Parse a single-relation read request and render it, panicking on parse/render
/// failure. Returns `(sql, params)`.
fn render(relation: &str, params: &[(String, String)]) -> (String, Vec<QueryParam>) {
    let plan = parse_select_request(relation, params, None, None).expect("parse_select_request");
    plan.render().expect("render")
}

/// A `TextArray` param from owned `&str` elements — test-readability helper.
fn text_array(items: &[&str]) -> QueryParam {
    QueryParam::TextArray(items.iter().map(|s| (*s).to_owned()).collect())
}

// ---------------------------------------------------------------------------
// 1. IN-list edge forms
// ---------------------------------------------------------------------------

#[test]
fn empty_in_list_matches_no_rows() {
    // `?id=in.()` → `id = ANY($1)` with an empty text[] (matches no rows).
    let (sql, params) = render("t", &[p("id", "in.()")]);
    assert_eq!(sql, "SELECT * FROM t WHERE id = ANY($1)");
    assert_eq!(params, vec![QueryParam::TextArray(vec![])]);
}

#[test]
fn literal_null_in_in_list_is_string() {
    // PostgREST treats `null` inside an in-list as the literal string "null",
    // NOT SQL NULL. The element must be exactly the string "null".
    let (sql, params) = render("t", &[p("state", "in.(active,null)")]);
    assert_eq!(sql, "SELECT * FROM t WHERE state = ANY($1)");
    assert_eq!(params, vec![text_array(&["active", "null"])]);
    // Prove it is the literal string, not a Null variant.
    match &params[0] {
        QueryParam::TextArray(items) => {
            assert_eq!(items[1], "null");
            assert_ne!(QueryParam::Null, QueryParam::TextArray(items.clone()));
        }
        other => panic!("expected TextArray, got {other:?}"),
    }
}

#[test]
fn quoted_comma_in_in_list_via_request() {
    // `?tag=in.("a,b",c)` → the quoted element protects its comma.
    let (sql, params) = render("t", &[p("tag", r#"in.("a,b",c)"#)]);
    assert_eq!(sql, "SELECT * FROM t WHERE tag = ANY($1)");
    assert_eq!(params, vec![text_array(&["a,b", "c"])]);
}

#[test]
fn composite_row_value_in_list_keeps_tuples() {
    // FIX (A): `?id=in.((1,2),(3,4))` must keep the two row-value tuples intact.
    // Before the paren-depth fix this produced 4 bogus elements.
    let (sql, params) = render("t", &[p("id", "in.((1,2),(3,4))")]);
    assert_eq!(sql, "SELECT * FROM t WHERE id = ANY($1)");
    assert_eq!(params, vec![text_array(&["(1,2)", "(3,4)"])]);
}

#[test]
fn in_list_nested_paren_single_tuple() {
    // Guards the "strip one outer pair" path: `in.((1,2))` is a single-element list
    // whose only element is the tuple `(1,2)` — the inner comma is at depth 1.
    let (sql, params, next) = render_condition("id", Operator::In, "((1,2))", false, None, None, 1)
        .expect("render_condition");
    assert_eq!(sql, "id = ANY($1)");
    assert_eq!(params, vec![text_array(&["(1,2)"])]);
    assert_eq!(next, 2);
}

#[test]
fn not_in_list_negates() {
    // `?id=not.in.(1,2)` → `NOT (id = ANY($1))`.
    let (sql, params) = render("t", &[p("id", "not.in.(1,2)")]);
    assert_eq!(sql, "SELECT * FROM t WHERE NOT (id = ANY($1))");
    assert_eq!(params, vec![text_array(&["1", "2"])]);
}

// ---------------------------------------------------------------------------
// 2. NULL semantics via `is`
// ---------------------------------------------------------------------------

#[test]
fn is_null_inlines_no_bind() {
    // `?deleted_at=is.null` inlines a validated literal — zero binds.
    let (sql, params) = render("t", &[p("deleted_at", "is.null")]);
    assert_eq!(sql, "SELECT * FROM t WHERE deleted_at IS NULL");
    assert!(params.is_empty());

    // `?deleted_at=not.is.null` → `NOT (deleted_at IS NULL)`.
    let (sql, params) = render("t", &[p("deleted_at", "not.is.null")]);
    assert_eq!(sql, "SELECT * FROM t WHERE NOT (deleted_at IS NULL)");
    assert!(params.is_empty());
}

// ---------------------------------------------------------------------------
// 3. Reserved-char / Unicode values are carried verbatim in a bind
// ---------------------------------------------------------------------------

#[test]
fn reserved_char_value_is_bound_not_interpolated() {
    // A value laced with SQL metacharacters must land in a bind param unchanged and
    // the SQL must contain only the `$1` placeholder — no quote, semicolon, or dash.
    let evil = "O'Brien); DROP TABLE t;--";
    let (sql, params) = render("t", &[p("name", &format!("eq.{evil}"))]);
    assert_eq!(sql, "SELECT * FROM t WHERE name = $1");
    assert_eq!(params, vec![QueryParam::Text(evil.to_owned())]);
    // The dangerous characters live only in the param, never the SQL text.
    assert!(!sql.contains('\''));
    assert!(!sql.contains(';'));
    assert!(!sql.contains("--"));
    assert!(!sql.contains("DROP"));
}

#[test]
fn unicode_value_preserved_in_bind() {
    let (sql, params) = render("t", &[p("bio", "eq.café☕")]);
    assert_eq!(sql, "SELECT * FROM t WHERE bio = $1");
    assert_eq!(params, vec![QueryParam::Text("café☕".to_owned())]);
}

#[test]
fn unicode_column_rejected() {
    // A non-ASCII COLUMN name is rejected because `parse_column_ref` (invoked at
    // render time) runs the base identifier through the ASCII-only safe-identifier
    // check. Parsing succeeds — the column is only validated when the leaf renders —
    // but rendering surfaces `FilterError::Ident(IdentError::Unsafe(_))`, so the
    // value never reaches SQL.
    let plan = parse_select_request("t", &[p("café", "eq.1")], None, None)
        .expect("parse defers column validation to render");
    let err = plan.render().unwrap_err();
    assert!(
        matches!(
            err,
            fdb_query::FilterError::Ident(fdb_query::IdentError::Unsafe(_))
        ),
        "expected FilterError::Ident(Unsafe(_)), got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// 4. Pagination edge forms
// ---------------------------------------------------------------------------

#[test]
fn limit_zero_renders_limit_0() {
    // `?limit=0` is valid and yields exactly zero rows.
    let (sql, _) = render("t", &[p("limit", "0")]);
    assert!(sql.ends_with(" LIMIT 0"), "got: {sql}");
    assert_eq!(sql, "SELECT * FROM t LIMIT 0");
}

#[test]
fn offset_u64_max_ok() {
    // u64::MAX offset is representable and rendered inline.
    let (sql, _) = render("t", &[p("offset", "18446744073709551615")]);
    assert!(sql.contains(" OFFSET 18446744073709551615"), "got: {sql}");
}

#[test]
fn offset_overflow_is_bad_number() {
    // One past u64::MAX overflows the parse and becomes a 400.
    let err =
        parse_select_request("t", &[p("offset", "18446744073709551616")], None, None).unwrap_err();
    assert!(matches!(err, ParseError::BadNumber(_)), "got: {err:?}");
}

// ---------------------------------------------------------------------------
// 5. Negation + quantifier composition
// ---------------------------------------------------------------------------

#[test]
fn not_eq_any_quantifier_composes() {
    // `?id=not.eq(any).(1,2,3)` → `NOT (id = ANY($1))`.
    let (sql, params) = render("t", &[p("id", "not.eq(any).(1,2,3)")]);
    assert_eq!(sql, "SELECT * FROM t WHERE NOT (id = ANY($1))");
    assert_eq!(params, vec![text_array(&["1", "2", "3"])]);
}

#[test]
fn not_like_all_quantifier() {
    // `?p=not.like(all).(a%,b%)` → `NOT (p LIKE ALL($1))`.
    let (sql, params) = render("t", &[p("p", "not.like(all).(a%,b%)")]);
    assert_eq!(sql, "SELECT * FROM t WHERE NOT (p LIKE ALL($1))");
    assert_eq!(params, vec![text_array(&["a%", "b%"])]);
}

#[test]
fn quantifier_on_in_and_is_rejected() {
    // A quantifier is meaningless on `in`/`is` and must error.
    let err = render_condition(
        "c",
        Operator::In,
        "(1,2)",
        false,
        Some(Quantifier::Any),
        None,
        1,
    )
    .unwrap_err();
    assert!(
        matches!(err, RenderError::QuantifierNotAllowed("in")),
        "got: {err:?}"
    );

    let err = render_condition(
        "c",
        Operator::Is,
        "null",
        false,
        Some(Quantifier::Any),
        None,
        1,
    )
    .unwrap_err();
    assert!(
        matches!(err, RenderError::QuantifierNotAllowed("is")),
        "got: {err:?}"
    );
}

// ---------------------------------------------------------------------------
// 6. JSON path + cast interaction
// ---------------------------------------------------------------------------

#[test]
fn json_path_cast_interaction() {
    // `?data->>age::int=gte.18` → `(data ->> 'age')::int >= $1`.
    let (sql, params) = render("t", &[p("data->>age::int", "gte.18")]);
    assert_eq!(sql, "SELECT * FROM t WHERE (data ->> 'age')::int >= $1");
    assert_eq!(params, vec![QueryParam::Text("18".to_owned())]);
}

#[test]
fn json_path_no_cast() {
    // `?data->>role=eq.admin` → `data ->> 'role' = $1`.
    let (sql, params) = render("t", &[p("data->>role", "eq.admin")]);
    assert_eq!(sql, "SELECT * FROM t WHERE data ->> 'role' = $1");
    assert_eq!(params, vec![QueryParam::Text("admin".to_owned())]);
}

// ---------------------------------------------------------------------------
// 7. ORDER BY over JSON paths
// ---------------------------------------------------------------------------

#[test]
fn order_by_json_path_with_nulls() {
    // A JSON key contains no '.', so `data->>ts.desc.nullslast` tokenizes cleanly
    // into column `data->>ts`, direction `desc`, nulls `nullslast`.
    let o = Order::parse("data->>ts.desc.nullslast").expect("order parse");
    assert_eq!(o.to_sql(), "ORDER BY data ->> 'ts' DESC NULLS LAST");
}

#[test]
fn order_by_json_array_index_default_asc() {
    // `?order=meta->0` — no direction token → default ASC.
    let o = Order::parse("meta->0").expect("order parse");
    assert_eq!(o.to_sql(), "ORDER BY meta -> 0 ASC");
}

#[test]
fn order_by_json_path_no_direction_default_asc() {
    let o = Order::parse("data->>ts").expect("order parse");
    assert_eq!(o.to_sql(), "ORDER BY data ->> 'ts' ASC");
}

// ---------------------------------------------------------------------------
// 8. Range header edge forms + precedence over query params
// ---------------------------------------------------------------------------

#[test]
fn range_inclusive_and_single() {
    // `Range: 0-24` (inclusive) → LIMIT 25 OFFSET 0.
    assert_eq!(
        Limits::from_range_header("0-24").expect("range"),
        Limits {
            limit: Some(25),
            offset: Some(0)
        }
    );
    // `Range: 0-0` → a single row.
    assert_eq!(
        Limits::from_range_header("0-0").expect("range"),
        Limits {
            limit: Some(1),
            offset: Some(0)
        }
    );
}

#[test]
fn range_open_ended_offset_only() {
    // `Range: 50-` → offset only, no limit.
    assert_eq!(
        Limits::from_range_header("50-").expect("range"),
        Limits {
            limit: None,
            offset: Some(50)
        }
    );
}

#[test]
fn range_inverted_and_malformed() {
    // Inverted range at the header layer.
    assert!(Limits::from_range_header("24-0").is_err());
    // Malformed forms at the header layer.
    assert!(Limits::from_range_header("garbage").is_err());
    assert!(Limits::from_range_header("-5").is_err());
    assert!(Limits::from_range_header("5").is_err());

    // Through the request path, each surfaces as a 400 BadRange.
    for bad in ["24-0", "garbage", "-5", "5"] {
        let err = parse_select_request("t", &[], Some(bad), None).unwrap_err();
        assert!(matches!(err, ParseError::BadRange(_)), "{bad:?} → {err:?}");
    }
}

#[test]
fn range_header_overrides_query_params() {
    // Both limit/offset params AND a Range header present → the Range header wins.
    let plan = parse_select_request(
        "t",
        &[p("limit", "5"), p("offset", "5")],
        Some("0-24"),
        None,
    )
    .expect("parse");
    let (sql, _) = plan.render().expect("render");
    assert_eq!(sql, "SELECT * FROM t LIMIT 25 OFFSET 0");
}

// ---------------------------------------------------------------------------
// 9. Group-member prefix collision (FIX B) + nesting still works
// ---------------------------------------------------------------------------

#[test]
fn group_member_column_starting_with_and_is_leaf() {
    // FIX (B): `?and=(android.eq.1)` — the column `android` merely starts with the
    // text "and". It must parse as a leaf, not a spuriously-nested and-group.
    let (sql, params) = render("t", &[p("and", "(android.eq.1)")]);
    assert_eq!(sql, "SELECT * FROM t WHERE android = $1");
    assert_eq!(params, vec![QueryParam::Text("1".to_owned())]);
}

#[test]
fn group_member_column_starting_with_or_is_leaf() {
    // Same class of bug for a column starting with "or" (e.g. `origin`).
    let (sql, params) = render("t", &[p("and", "(origin.eq.x)")]);
    assert_eq!(sql, "SELECT * FROM t WHERE origin = $1");
    assert_eq!(params, vec![QueryParam::Text("x".to_owned())]);
}

#[test]
fn nested_and_group_still_detected_after_fix() {
    // Guard: the fix must NOT break genuine nesting. `or(...)` immediately followed
    // by `(` is still a real nested group.
    let (sql, params) = render("t", &[p("and", "(a.gt.1,or(b.eq.2,c.eq.3))")]);
    assert_eq!(sql, "SELECT * FROM t WHERE (a > $1 AND (b = $2 OR c = $3))");
    assert_eq!(
        params,
        vec![
            QueryParam::Text("1".to_owned()),
            QueryParam::Text("2".to_owned()),
            QueryParam::Text("3".to_owned()),
        ]
    );
}

// ---------------------------------------------------------------------------
// 10. Select projection edge (JSON + rename) via the public parser
// ---------------------------------------------------------------------------

#[test]
fn select_json_and_rename_projection() {
    // Sanity that Select::parse survives JSON paths + aliases end to end.
    let s = Select::parse("id,full_name:name,data->>email").expect("select");
    assert_eq!(s.to_sql(), "id, name AS full_name, data ->> 'email'");
}
