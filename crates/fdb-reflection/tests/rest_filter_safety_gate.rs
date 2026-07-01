//! Security gate: `test_rest_select_with_eq_filter` and the 12-operator +
//! SQL-injection coverage for the REST filter compiler.
//!
//! P2-carried gate (RFC-FORGE §3.3 / G6). Two guarantees:
//!   1. Every one of the 12 PostgREST filter operators compiles to the expected
//!      SQL fragment, with the VALUE bound (`$n`) and never interpolated.
//!   2. Every column identifier that reaches SQL is validated by
//!      `forge_domain::is_safe_identifier`; a battery of injection vectors is
//!      rejected before it can be interpolated.

use fdb_reflection::compilers::filters::{FilterOp, build_where, parse_filter};
use forge_domain::is_safe_identifier;

/// The named gate: an `eq` filter parameterizes its value and renders `col = $1`.
#[test]
fn test_rest_select_with_eq_filter() {
    let f = parse_filter("status", "eq.active").expect("eq parses");
    assert_eq!(f.op, FilterOp::Eq);

    let wc = build_where(&[f], 1);
    assert_eq!(wc.sql, "WHERE status = $1", "eq must render a bound placeholder");
    assert_eq!(
        wc.binds,
        vec!["active".to_owned()],
        "the value must be BOUND, not interpolated into the SQL text"
    );
    // The literal value must NOT appear in the SQL string.
    assert!(
        !wc.sql.contains("active"),
        "value leaked into SQL text (interpolation, not binding): {}",
        wc.sql
    );
}

/// Each of the 12 operators renders to its exact SQL fragment.
#[test]
fn all_twelve_operators_render_expected_sql() {
    let cases: &[(&str, FilterOp, &str)] = &[
        ("eq.1", FilterOp::Eq, "WHERE c = $1"),
        ("neq.1", FilterOp::Neq, "WHERE c <> $1"),
        ("gt.1", FilterOp::Gt, "WHERE c > $1"),
        ("gte.1", FilterOp::Gte, "WHERE c >= $1"),
        ("lt.1", FilterOp::Lt, "WHERE c < $1"),
        ("lte.1", FilterOp::Lte, "WHERE c <= $1"),
        ("like.a%", FilterOp::Like, "WHERE c LIKE $1"),
        ("ilike.a%", FilterOp::Ilike, "WHERE c ILIKE $1"),
        ("in.1,2", FilterOp::In, "WHERE c = ANY($1)"),
        ("is.null", FilterOp::Is, "WHERE c IS NULL"),
        ("cs.{1}", FilterOp::Cs, "WHERE c @> $1"),
        ("cd.{1}", FilterOp::Cd, "WHERE c <@ $1"),
    ];
    for (raw, op, expected_sql) in cases {
        let f = parse_filter("c", raw).unwrap_or_else(|e| panic!("parse {raw}: {e}"));
        assert_eq!(f.op, *op, "operator mismatch for {raw}");
        let wc = build_where(std::slice::from_ref(&f), 1);
        assert_eq!(&wc.sql, expected_sql, "SQL mismatch for {raw}");
    }
}

/// Scalar/containment operators bind their value; only `is` inlines a
/// validated literal, and `in` binds an array parameter.
#[test]
fn values_are_bound_not_interpolated() {
    // Scalar: value bound.
    let scalar = build_where(&[parse_filter("age", "gte.18").unwrap()], 1);
    assert_eq!(scalar.binds, vec!["18".to_owned()]);
    assert!(!scalar.sql.contains("18"));

    // In: single array bind, value not in SQL text.
    let in_wc = build_where(&[parse_filter("id", "in.1,2,3").unwrap()], 1);
    assert_eq!(in_wc.sql, "WHERE id = ANY($1)");
    assert_eq!(in_wc.binds.len(), 1);
    assert!(!in_wc.sql.contains('3'));

    // Is: NULL is a validated literal (null/true/false only), binds nothing.
    let is_wc = build_where(&[parse_filter("deleted_at", "is.null").unwrap()], 1);
    assert_eq!(is_wc.sql, "WHERE deleted_at IS NULL");
    assert!(is_wc.binds.is_empty());
}

/// SQL-injection vectors as COLUMN names must be rejected by
/// `is_safe_identifier` (and therefore by `parse_filter`), never reaching SQL.
#[test]
fn injection_column_identifiers_are_rejected() {
    let injections = [
        "id; DROP TABLE users--",
        "id-- ",
        "id UNION SELECT",
        "id' OR '1'='1",
        "pg_sleep(10)",
        "col name",             // whitespace
        "1col",                 // starts with digit
        &"a".repeat(64),        // over NAMEDATALEN-1 (63)
    ];
    for bad in injections {
        assert!(
            !is_safe_identifier(bad),
            "is_safe_identifier must reject injection vector: {bad:?}"
        );
        assert!(
            parse_filter(bad, "eq.1").is_err(),
            "parse_filter must reject unsafe column: {bad:?}"
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
    let wc = build_where(&[parse_filter("name", "eq.Robert'); DROP TABLE--").unwrap()], 1);
    assert_eq!(wc.sql, "WHERE name = $1");
    assert_eq!(wc.binds, vec!["Robert'); DROP TABLE--".to_owned()]);
    assert!(!wc.sql.contains("DROP"), "dangerous value must be bound, not in SQL");
}
