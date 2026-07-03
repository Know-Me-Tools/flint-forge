//! Security gate: `test_rest_select_with_eq_filter` and operator + SQL-injection
//! coverage for the REST filter path (now backed by the shared `fdb-query`
//! translator via the `compilers::filters` bridge).
//!
//! P2-carried gate (RFC-FORGE §3.3 / G6). Two guarantees:
//!   1. Filter operators compile to the expected SQL fragment, with the VALUE
//!      bound (`$n`) and never interpolated.
//!   2. Every column identifier that reaches SQL is validated; a battery of
//!      injection vectors is rejected before it can be interpolated.

use std::collections::HashMap;

use fdb_query::QueryParam;
use fdb_reflection::compilers::filters::{parse_filter_tree, render_where};
use forge_domain::is_safe_identifier;

/// One-filter param map helper.
fn one(column: &str, raw: &str) -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert(column.to_owned(), raw.to_owned());
    m
}

/// Render a single `column=raw` filter to its `WHERE` clause.
fn where_of(column: &str, raw: &str) -> Result<fdb_reflection::compilers::filters::WhereClause, String> {
    let tree = parse_filter_tree(&one(column, raw)).map_err(|e| e.to_string())?;
    render_where(&tree, 1)
}

/// The named gate: an `eq` filter parameterizes its value and renders `col = $1`.
#[test]
fn test_rest_select_with_eq_filter() {
    let wc = where_of("status", "eq.active").expect("eq renders");
    assert_eq!(wc.sql, "WHERE status = $1", "eq must render a bound placeholder");
    assert_eq!(
        wc.binds,
        vec![QueryParam::Text("active".to_owned())],
        "the value must be BOUND, not interpolated into the SQL text"
    );
    assert!(
        !wc.sql.contains("active"),
        "value leaked into SQL text (interpolation, not binding): {}",
        wc.sql
    );
}

/// Core operators render to their exact single-leaf SQL fragment with the value
/// bound (`is` inlines a validated literal; `in` binds an array).
#[test]
fn operators_render_expected_sql() {
    let cases: &[(&str, &str)] = &[
        ("eq.1", "WHERE c = $1"),
        ("neq.1", "WHERE c <> $1"),
        ("gt.1", "WHERE c > $1"),
        ("gte.1", "WHERE c >= $1"),
        ("lt.1", "WHERE c < $1"),
        ("lte.1", "WHERE c <= $1"),
        ("like.a%", "WHERE c LIKE $1"),
        ("ilike.a%", "WHERE c ILIKE $1"),
        ("in.1,2", "WHERE c = ANY($1)"),
        ("is.null", "WHERE c IS NULL"),
        ("cs.{1}", "WHERE c @> $1"),
        ("cd.{1}", "WHERE c <@ $1"),
    ];
    for (raw, expected_sql) in cases {
        let wc = where_of("c", raw).unwrap_or_else(|e| panic!("render {raw}: {e}"));
        assert_eq!(&wc.sql, expected_sql, "SQL mismatch for {raw}");
    }
}

/// Scalar/containment operators bind their value; `is` inlines a validated
/// literal and binds nothing; `in` binds a single array parameter.
#[test]
fn values_are_bound_not_interpolated() {
    let scalar = where_of("age", "gte.18").unwrap();
    assert_eq!(scalar.binds, vec![QueryParam::Text("18".to_owned())]);
    assert!(!scalar.sql.contains("18"));

    let in_wc = where_of("id", "in.1,2,3").unwrap();
    assert_eq!(in_wc.sql, "WHERE id = ANY($1)");
    assert_eq!(in_wc.binds.len(), 1);
    assert!(matches!(in_wc.binds[0], QueryParam::TextArray(_)));
    assert!(!in_wc.sql.contains('3'));

    let is_wc = where_of("deleted_at", "is.null").unwrap();
    assert_eq!(is_wc.sql, "WHERE deleted_at IS NULL");
    assert!(is_wc.binds.is_empty());
}

/// SQL-injection vectors as COLUMN names must be rejected before reaching SQL —
/// the `fdb-query` renderer validates the identifier and errors out.
#[test]
fn injection_column_identifiers_are_rejected() {
    let injections = [
        "id; DROP TABLE users--",
        "id-- ",
        "id UNION SELECT",
        "id' OR '1'='1",
        "pg_sleep(10)",
        "col name",      // whitespace
        "1col",          // starts with digit
        &"a".repeat(64), // over NAMEDATALEN-1 (63)
    ];
    for bad in injections {
        assert!(
            !is_safe_identifier(bad),
            "is_safe_identifier must reject injection vector: {bad:?}"
        );
        // The tree parses (op.value is well-formed), but render validates the
        // column and rejects it — the unsafe name never reaches SQL.
        let rendered = where_of(bad, "eq.1");
        assert!(
            rendered.is_err(),
            "render must reject unsafe column: {bad:?}"
        );
    }
}

/// Valid identifiers (including `schema.table` and words that merely CONTAIN a
/// keyword) are still accepted — the reject list must not be over-broad.
#[test]
fn valid_identifiers_still_accepted() {
    for good in ["id", "user_id", "public.items", "selected", "_private"] {
        assert!(is_safe_identifier(good), "should accept {good:?}");
    }
    // A safe column with a value that merely looks dangerous is bound, not run.
    let wc = where_of("name", "eq.Robert'); DROP TABLE--").unwrap();
    assert_eq!(wc.sql, "WHERE name = $1");
    assert_eq!(
        wc.binds,
        vec![QueryParam::Text("Robert'); DROP TABLE--".to_owned())]
    );
    assert!(!wc.sql.contains("DROP"), "dangerous value must be bound, not in SQL");
}
