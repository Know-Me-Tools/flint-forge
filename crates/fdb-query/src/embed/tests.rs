//! Unit tests for the embed module (parsing, resolution, rendering).
#![allow(clippy::similar_names)] // test locals `sel` (EmbedSelect) and `sql` are both clear

use super::*;
use crate::clause::Select;
use crate::param::QueryParam;


// --- schema fixtures -------------------------------------------------

/// customers 1—* orders (orders.customer_id -> customers.id); orders *—1 customers.
fn schema_basic() -> EmbedSchema {
    let customers = TableDesc::new()
        .with_column("id")
        .with_column("name")
        .with_column("tier")
        .with_fk(FkEdge {
            fk_name: "fk_orders_customer".into(),
            from_table: "orders".into(),
            from_col: "customer_id".into(),
            to_table: "customers".into(),
            to_col: "id".into(),
            cardinality: Cardinality::ToMany,
        });
    let orders = TableDesc::new()
        .with_column("id")
        .with_column("customer_id")
        .with_column("status")
        .with_column("total")
        .with_column("created_at")
        .with_fk(FkEdge {
            fk_name: "fk_orders_customer".into(),
            from_table: "orders".into(),
            from_col: "customer_id".into(),
            to_table: "customers".into(),
            to_col: "id".into(),
            cardinality: Cardinality::ToOne,
        });
    EmbedSchema::new()
        .with_table("customers", customers)
        .with_table("orders", orders)
}

fn resolve(raw: &str, parent: &str, schema: &EmbedSchema) -> (EmbedSelect, Vec<ResolvedEmbed>) {
    let sel = parse_embed_select(raw).expect("parse");
    let resolved = resolve_embeds(&sel, parent, "p", schema).expect("resolve");
    (sel, resolved)
}

// --- parsing ---------------------------------------------------------

#[test]
fn parses_scalar_and_embed_tokens() {
    let sel = parse_embed_select("*,orders(id,total)").unwrap();
    assert_eq!(sel.columns.len(), 1);
    assert!(sel.columns[0].star);
    assert_eq!(sel.embeds.len(), 1);
    assert_eq!(sel.embeds[0].target, "orders");
    assert_eq!(sel.embeds[0].select.columns.len(), 2);
}

#[test]
fn parses_alias_fk_inner_and_spread() {
    let sel = parse_embed_select("recent:orders!inner(id),...customer(name)").unwrap();
    let a = &sel.embeds[0];
    assert_eq!(a.alias.as_deref(), Some("recent"));
    assert_eq!(a.target, "orders");
    assert_eq!(a.join, JoinKind::Inner);
    assert!(!a.spread);
    let b = &sel.embeds[1];
    assert!(b.spread);
    assert_eq!(b.target, "customer");

    let fk = parse_embed_select("orders!fk_x(id)").unwrap();
    assert_eq!(fk.embeds[0].fk_hint.as_deref(), Some("fk_x"));
    assert_eq!(fk.embeds[0].join, JoinKind::Left);
}

#[test]
fn malformed_embed_rejected() {
    assert!(matches!(
        parse_embed_select("orders(id").unwrap_err(),
        EmbedError::MalformedEmbed(_)
    ));
}

#[test]
fn malicious_alias_fails_validation_at_parse() {
    let err = parse_embed_select("x);DROP:orders(id)").unwrap_err();
    assert!(matches!(err, EmbedError::Ident(_)));
}

// --- rendering: to-many ---------------------------------------------

#[test]
fn to_many_embed_renders_json_agg_coalesced_empty() {
    let schema = schema_basic();
    let (_, resolved) = resolve("*,orders(id,total)", "customers", &schema);
    let base = Select::default();
    let (sql, params, next) = render_projection(&base, &resolved, 1).unwrap();
    assert_eq!(
        sql,
        "*, COALESCE((SELECT json_agg(json_build_object('id', orders_1.id, 'total', orders_1.total)) \
         FROM orders orders_1 WHERE p.id = orders_1.customer_id), '[]'::json) AS orders"
    );
    assert!(params.is_empty());
    assert_eq!(next, 1);
}

#[test]
fn to_many_star_expands_child_columns() {
    let schema = schema_basic();
    let (_, resolved) = resolve("*,orders(*)", "customers", &schema);
    let base = Select::default();
    let (sql, _, _) = render_projection(&base, &resolved, 1).unwrap();
    assert!(sql.contains("json_build_object('id', orders_1.id, 'customer_id', orders_1.customer_id, 'status', orders_1.status, 'total', orders_1.total, 'created_at', orders_1.created_at)"));
}

// --- rendering: to-one ----------------------------------------------

#[test]
fn to_one_embed_renders_json_build_object_limit_1() {
    let schema = schema_basic();
    let (_, resolved) = resolve("id,customers(name)", "orders", &schema);
    let base = Select::parse("id").unwrap();
    let (sql, _, _) = render_projection(&base, &resolved, 1).unwrap();
    assert_eq!(
        sql,
        "id, (SELECT json_build_object('name', customers_1.name) FROM customers customers_1 \
         WHERE p.customer_id = customers_1.id LIMIT 1) AS customers"
    );
}

// --- FK disambiguation ----------------------------------------------

fn schema_two_fks() -> EmbedSchema {
    // customers referenced by orders via billing_id and shipping_id.
    let customers = TableDesc::new().with_column("id").with_column("name");
    let orders = TableDesc::new()
        .with_column("id")
        .with_column("billing_id")
        .with_column("shipping_id")
        .with_fk(FkEdge {
            fk_name: "fk_billing".into(),
            from_table: "orders".into(),
            from_col: "billing_id".into(),
            to_table: "customers".into(),
            to_col: "id".into(),
            cardinality: Cardinality::ToOne,
        })
        .with_fk(FkEdge {
            fk_name: "fk_shipping".into(),
            from_table: "orders".into(),
            from_col: "shipping_id".into(),
            to_table: "customers".into(),
            to_col: "id".into(),
            cardinality: Cardinality::ToOne,
        });
    EmbedSchema::new()
        .with_table("customers", customers)
        .with_table("orders", orders)
}

#[test]
fn fk_disambiguation_picks_named_edge() {
    let schema = schema_two_fks();
    let (_, resolved) = resolve("id,customers!fk_shipping(name)", "orders", &schema);
    assert_eq!(resolved[0].edge.fk_name, "fk_shipping");
    let base = Select::parse("id").unwrap();
    let (sql, _, _) = render_projection(&base, &resolved, 1).unwrap();
    assert!(sql.contains("WHERE p.shipping_id = customers_1.id"));
}

#[test]
fn ambiguous_fk_lists_candidates() {
    let schema = schema_two_fks();
    let sel = parse_embed_select("id,customers(name)").unwrap();
    let err = resolve_embeds(&sel, "orders", "p", &schema).unwrap_err();
    match err {
        EmbedError::AmbiguousFk { candidates, .. } => {
            assert!(candidates.contains(&"fk_billing".to_owned()));
            assert!(candidates.contains(&"fk_shipping".to_owned()));
        }
        other => panic!("expected AmbiguousFk, got {other:?}"),
    }
}

#[test]
fn unknown_fk_name_errors() {
    let schema = schema_two_fks();
    let sel = parse_embed_select("id,customers!fk_nope(name)").unwrap();
    assert!(matches!(
        resolve_embeds(&sel, "orders", "p", &schema).unwrap_err(),
        EmbedError::UnknownFkName(_)
    ));
}

// --- !inner ----------------------------------------------------------

#[test]
fn inner_join_adds_exists_predicate() {
    let schema = schema_basic();
    let (_, resolved) = resolve("*,orders!inner(id)", "customers", &schema);
    let (preds, params, next) = render_inner_guards(&resolved, 1).unwrap();
    assert_eq!(preds.len(), 1);
    assert_eq!(
        preds[0],
        "EXISTS (SELECT 1 FROM orders orders_1 WHERE p.id = orders_1.customer_id)"
    );
    assert!(params.is_empty());
    assert_eq!(next, 1);
}

#[test]
fn left_join_produces_no_exists() {
    let schema = schema_basic();
    let (_, resolved) = resolve("*,orders(id)", "customers", &schema);
    let (preds, _, _) = render_inner_guards(&resolved, 1).unwrap();
    assert!(preds.is_empty());
}

// --- embedded filter routing ----------------------------------------

#[test]
fn embedded_filter_scopes_to_subselect_and_binds_param() {
    let schema = schema_basic();
    let mut sel = parse_embed_select("*,orders(id)").unwrap();
    assert!(route_embedded_param(&mut sel, "orders.status", "eq.shipped").unwrap());
    let resolved = resolve_embeds(&sel, "customers", "p", &schema).unwrap();
    let base = Select::default();
    let (sql, params, next) = render_projection(&base, &resolved, 1).unwrap();
    assert!(
        sql.contains("WHERE p.id = orders_1.customer_id AND orders_1.status = $1"),
        "got: {sql}"
    );
    assert_eq!(params, vec![QueryParam::Text("shipped".into())]);
    assert_eq!(next, 2);
}

#[test]
fn embedded_filter_also_appears_in_inner_exists() {
    let schema = schema_basic();
    let mut sel = parse_embed_select("*,orders!inner(id)").unwrap();
    route_embedded_param(&mut sel, "orders.status", "eq.active").unwrap();
    let resolved = resolve_embeds(&sel, "customers", "p", &schema).unwrap();
    let (preds, params, _) = render_inner_guards(&resolved, 1).unwrap();
    assert_eq!(
        preds[0],
        "EXISTS (SELECT 1 FROM orders orders_1 WHERE p.id = orders_1.customer_id AND orders_1.status = $1)"
    );
    assert_eq!(params, vec![QueryParam::Text("active".into())]);
}

// --- embedded order --------------------------------------------------

#[test]
fn embedded_order_renders_inside_json_agg() {
    let schema = schema_basic();
    let mut sel = parse_embed_select("*,orders(id)").unwrap();
    route_embedded_param(&mut sel, "order", "orders.created_at.desc").unwrap();
    let resolved = resolve_embeds(&sel, "customers", "p", &schema).unwrap();
    let base = Select::default();
    let (sql, _, _) = render_projection(&base, &resolved, 1).unwrap();
    assert!(
        sql.contains("json_agg(json_build_object('id', orders_1.id) ORDER BY orders_1.created_at DESC)"),
        "got: {sql}"
    );
}

// --- embedded pagination --------------------------------------------

#[test]
fn embedded_limit_wraps_derived_table() {
    let schema = schema_basic();
    let mut sel = parse_embed_select("*,orders(id)").unwrap();
    route_embedded_param(&mut sel, "orders.limit", "5").unwrap();
    let resolved = resolve_embeds(&sel, "customers", "p", &schema).unwrap();
    let base = Select::default();
    let (sql, _, _) = render_projection(&base, &resolved, 1).unwrap();
    assert!(
        sql.contains(
            "COALESCE((SELECT json_agg(json_build_object('id', sub.id)) \
             FROM (SELECT * FROM orders orders_1 WHERE p.id = orders_1.customer_id LIMIT 5) sub), '[]'::json) AS orders"
        ),
        "got: {sql}"
    );
}

// --- spread ----------------------------------------------------------

#[test]
fn spread_to_one_flattens_columns() {
    let schema = schema_basic();
    let (_, resolved) = resolve("id,...customers(name,tier)", "orders", &schema);
    let base = Select::parse("id").unwrap();
    let (sql, _, _) = render_projection(&base, &resolved, 1).unwrap();
    assert_eq!(
        sql,
        "id, (SELECT customers_1.name FROM customers customers_1 WHERE p.customer_id = customers_1.id LIMIT 1) AS name, \
         (SELECT customers_1.tier FROM customers customers_1 WHERE p.customer_id = customers_1.id LIMIT 1) AS tier"
    );
}

#[test]
fn spread_on_to_many_is_rejected() {
    let schema = schema_basic();
    let sel = parse_embed_select("*,...orders(total)").unwrap();
    assert!(matches!(
        resolve_embeds(&sel, "customers", "p", &schema).unwrap_err(),
        EmbedError::SpreadRequiresToOne(t) if t == "orders"
    ));
}

// --- nested ----------------------------------------------------------

fn schema_nested() -> EmbedSchema {
    let mut s = schema_basic();
    // orders 1—* items (items.order_id -> orders.id)
    let items = TableDesc::new()
        .with_column("id")
        .with_column("order_id")
        .with_column("sku")
        .with_fk(FkEdge {
            fk_name: "fk_items_order".into(),
            from_table: "items".into(),
            from_col: "order_id".into(),
            to_table: "orders".into(),
            to_col: "id".into(),
            cardinality: Cardinality::ToOne,
        });
    // register the reverse edge on orders so customers->orders->items traverses
    let orders = s.table("orders").unwrap().clone();
    let orders = TableDesc {
        columns: orders.columns,
        fks: {
            let mut f = orders.fks;
            f.push(FkEdge {
                fk_name: "fk_items_order".into(),
                from_table: "items".into(),
                from_col: "order_id".into(),
                to_table: "orders".into(),
                to_col: "id".into(),
                cardinality: Cardinality::ToMany,
            });
            f
        },
    };
    s = s.with_table("items", items).with_table("orders", orders);
    s
}

#[test]
fn nested_embed_recurses_with_distinct_aliases() {
    let schema = schema_nested();
    let (_, resolved) = resolve("*,orders(id,items(sku))", "customers", &schema);
    let base = Select::default();
    let (sql, _, _) = render_projection(&base, &resolved, 1).unwrap();
    // outer json_agg over orders; inner json_agg over items correlated on orders_1.
    assert!(sql.contains("FROM orders orders_1 WHERE p.id = orders_1.customer_id"), "got: {sql}");
    assert!(
        sql.contains("FROM items items_2 WHERE orders_1.id = items_2.order_id"),
        "got: {sql}"
    );
    assert!(sql.contains("'items', COALESCE((SELECT json_agg(json_build_object('sku', items_2.sku))"), "got: {sql}");
}

// --- index threading -------------------------------------------------

#[test]
fn index_threading_is_globally_monotonic() {
    // Two embeds (a to-many `orders` and a spread to-one via the nested
    // schema's `items`→`orders` to-one edge) each contribute one bound param;
    // the shared counter numbers them $1, $2 in projection emission order.
    let schema = schema_nested();
    let mut sel = parse_embed_select("*,orders(id),...orders2:orders(status)").unwrap();
    // First embed binds $1 (status=a); the spread `orders2` binds $2 (total).
    // Both target `orders`, which is to-one *from `items`* — but here parent is
    // `items`, so `orders` is to-one and a spread is legal.
    route_embedded_param(&mut sel, "orders.status", "eq.a").unwrap();
    route_embedded_param(&mut sel, "orders2.total", "gte.10").unwrap();
    let resolved = resolve_embeds(&sel, "items", "p", &schema).unwrap();
    let base = Select::default();
    let (_, params, next) = render_projection(&base, &resolved, 1).unwrap();
    // orders embed binds $1 (status), spread orders2 binds $2 (total).
    assert_eq!(params.len(), 2);
    assert_eq!(next, 3);
    assert_eq!(
        params,
        vec![QueryParam::Text("a".into()), QueryParam::Text("10".into())]
    );
}

// --- error paths -----------------------------------------------------

#[test]
fn unknown_relation_errors() {
    let schema = schema_basic();
    let sel = parse_embed_select("*,nope(id)").unwrap();
    assert!(matches!(
        resolve_embeds(&sel, "customers", "p", &schema).unwrap_err(),
        EmbedError::UnknownRelation(r) if r == "nope"
    ));
}

#[test]
fn unknown_embedded_column_errors() {
    let schema = schema_basic();
    let sel = parse_embed_select("*,orders(bogus)").unwrap();
    match resolve_embeds(&sel, "customers", "p", &schema).unwrap_err() {
        EmbedError::UnknownColumn { table, column } => {
            assert_eq!(table, "orders");
            assert_eq!(column, "bogus");
        }
        other => panic!("expected UnknownColumn, got {other:?}"),
    }
}

#[test]
fn no_fk_path_errors() {
    // customers with no linking FK to an unrelated table.
    let schema = EmbedSchema::new()
        .with_table("customers", TableDesc::new().with_column("id"))
        .with_table("widgets", TableDesc::new().with_column("id"));
    let sel = parse_embed_select("*,widgets(id)").unwrap();
    assert!(matches!(
        resolve_embeds(&sel, "customers", "p", &schema).unwrap_err(),
        EmbedError::NoFkPath { .. }
    ));
}

#[test]
fn embed_identifiers_never_interpolate_user_text() {
    // Alias-position injection is caught at parse.
    let err = parse_embed_select("orders(x);DROP:id)").unwrap_err();
    assert!(matches!(err, EmbedError::MalformedEmbed(_) | EmbedError::Ident(_)));
}

#[test]
fn resolve_embeds_rejects_unsafe_parent_alias_and_table() {
    // Regression: the top-level parent alias/table flow verbatim into correlation
    // predicates, so resolve_embeds MUST validate them (defense-in-depth) rather
    // than trust the caller. An injection payload in either position is rejected.
    let schema = schema_basic();
    let sel = parse_embed_select("*,orders(*)").expect("parse");

    let bad_alias = resolve_embeds(&sel, "customers", "p) UNION SELECT secret FROM v --", &schema);
    assert!(
        matches!(bad_alias, Err(EmbedError::Ident(_))),
        "unsafe parent_alias must be rejected, got {bad_alias:?}"
    );

    let bad_table = resolve_embeds(&sel, "customers; DROP TABLE users--", "p", &schema);
    assert!(
        matches!(bad_table, Err(EmbedError::Ident(_))),
        "unsafe parent_table must be rejected, got {bad_table:?}"
    );

    // The safe baseline still resolves.
    assert!(resolve_embeds(&sel, "customers", "p", &schema).is_ok());
}
