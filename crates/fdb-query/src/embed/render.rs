//! Schema-aware resolution and SQL rendering of embeds.
//!
//! Resolves each [`EmbedRequest`] to a [`ResolvedEmbed`] bound to a concrete
//! [`FkEdge`], then renders correlated `json_agg` / `json_build_object`
//! subselects, spread projections, and `!inner` `EXISTS` guards — all threading
//! one shared `$n` counter.

use crate::clause::Select;
use crate::filter::FilterTree;
use crate::ident::{parse_column_ref, validate_identifier, IdentError};
use crate::param::QueryParam;

use super::schema::{
    embed_output_name, Cardinality, EmbedError, EmbedRequest, EmbedSchema, EmbedSelect, FkEdge,
    JoinKind, ResolvedEmbed, ScalarCol, TableDesc,
};

/// Resolve every [`EmbedRequest`] under `sel` against the schema: pick the
/// [`FkEdge`] (honoring `!fk`), assign deterministic child aliases, validate
/// spread cardinality and referenced columns, and expand `*`.
///
/// # Errors
/// Returns [`EmbedError`] for unknown relations/columns, missing/ambiguous FKs,
/// or a spread on a to-many edge.
pub fn resolve_embeds(
    sel: &EmbedSelect,
    parent_table: &str,
    parent_alias: &str,
    schema: &EmbedSchema,
) -> Result<Vec<ResolvedEmbed>, EmbedError> {
    // Defense-in-depth: the top-level parent table/alias are caller-supplied and
    // flow verbatim into correlation predicates. Validate them here so an unsafe
    // alias can never reach SQL, matching how child aliases are safe by construction.
    crate::validate_identifier(parent_table)
        .map_err(|_| EmbedError::Ident(crate::IdentError::Unsafe(parent_table.to_owned())))?;
    crate::validate_identifier(parent_alias)
        .map_err(|_| EmbedError::Ident(crate::IdentError::Unsafe(parent_alias.to_owned())))?;
    let mut counter = 0usize;
    resolve_level(sel, parent_table, parent_alias, schema, &mut counter)
}

fn resolve_level(
    sel: &EmbedSelect,
    parent_table: &str,
    parent_alias: &str,
    schema: &EmbedSchema,
    counter: &mut usize,
) -> Result<Vec<ResolvedEmbed>, EmbedError> {
    let mut out = Vec::with_capacity(sel.embeds.len());
    for req in &sel.embeds {
        out.push(resolve_one(
            req,
            parent_table,
            parent_alias,
            schema,
            counter,
        )?);
    }
    Ok(out)
}

fn resolve_one(
    req: &EmbedRequest,
    parent_table: &str,
    parent_alias: &str,
    schema: &EmbedSchema,
    counter: &mut usize,
) -> Result<ResolvedEmbed, EmbedError> {
    let child_desc = schema
        .table(&req.target)
        .ok_or_else(|| EmbedError::UnknownRelation(req.target.clone()))?;

    let edge = pick_edge(parent_table, &req.target, req.fk_hint.as_deref(), schema)?;

    if req.spread && edge.cardinality == Cardinality::ToMany {
        return Err(EmbedError::SpreadRequiresToOne(req.target.clone()));
    }

    *counter += 1;
    let child_alias = format!("{}_{}", req.target, counter);

    // Expand `*` to concrete columns; validate explicit columns against the table.
    let columns = expand_columns(&req.select.columns, &req.target, child_desc)?;

    let children = resolve_level(&req.select, &req.target, &child_alias, schema, counter)?;

    Ok(ResolvedEmbed {
        request: req.clone(),
        edge,
        parent_alias: parent_alias.to_owned(),
        child_alias,
        columns,
        children,
        cast_hints: child_desc.cast_hints.clone(),
    })
}

/// Expand a projection list: `*` (or empty) → every table column; explicit
/// columns validated against `TableDesc.columns`.
fn expand_columns(
    cols: &[ScalarCol],
    table: &str,
    desc: &TableDesc,
) -> Result<Vec<ScalarCol>, EmbedError> {
    let wants_star = cols.is_empty() || cols.iter().any(|c| c.star);
    let mut out: Vec<ScalarCol> = Vec::new();
    if wants_star {
        for c in &desc.columns {
            out.push(ScalarCol {
                key: c.clone(),
                col_ref: c.clone(),
                star: false,
            });
        }
    }
    for c in cols {
        if c.star {
            continue;
        }
        let base = parse_column_ref(&c.col_ref)?.base().to_owned();
        if !desc.columns.iter().any(|col| col == &base) {
            return Err(EmbedError::UnknownColumn {
                table: table.to_owned(),
                column: base,
            });
        }
        if !out.iter().any(|o| o.key == c.key) {
            out.push(c.clone());
        }
    }
    Ok(out)
}

/// Pick the FK edge linking `parent_table` → `target`, honoring an optional
/// `!fk` hint. Considers edges in both directions between the two tables.
fn pick_edge(
    parent_table: &str,
    target: &str,
    fk_hint: Option<&str>,
    schema: &EmbedSchema,
) -> Result<FkEdge, EmbedError> {
    let mut candidates: Vec<FkEdge> = Vec::new();
    for desc in [schema.table(parent_table), schema.table(target)]
        .into_iter()
        .flatten()
    {
        for edge in &desc.fks {
            let links = (edge.from_table == parent_table && edge.to_table == target)
                || (edge.from_table == target && edge.to_table == parent_table);
            if links && !candidates.iter().any(|c| c.fk_name == edge.fk_name) {
                candidates.push(edge.clone());
            }
        }
    }

    if let Some(hint) = fk_hint {
        return candidates
            .into_iter()
            .find(|e| e.fk_name == hint)
            .ok_or_else(|| EmbedError::UnknownFkName(hint.to_owned()));
    }

    match candidates.len() {
        0 => Err(EmbedError::NoFkPath {
            from: parent_table.to_owned(),
            to: target.to_owned(),
        }),
        1 => Ok(candidates.into_iter().next().expect("len==1")),
        _ => Err(EmbedError::AmbiguousFk {
            from: parent_table.to_owned(),
            to: target.to_owned(),
            candidates: candidates.into_iter().map(|e| e.fk_name).collect(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// The FK correlation predicate `<parent>.<col> = <child>.<col>`. Both sides are
/// validated identifiers taken only from the [`FkEdge`]; no user text.
///
/// The edge is stored as `from_table.from_col -> to_table.to_col`. Whichever end
/// is the child (`child_table == edge.from_table` or `== edge.to_table`) selects
/// which validated column each alias contributes.
fn correlation_sql(
    edge: &FkEdge,
    parent_alias: &str,
    child_alias: &str,
    child_table: &str,
) -> Result<String, EmbedError> {
    let (parent_col, child_col) = if edge.from_table == child_table {
        // Child holds the FK (to-one from child's view / to-many for parent).
        (&edge.to_col, &edge.from_col)
    } else {
        (&edge.from_col, &edge.to_col)
    };
    let p = validate_identifier(parent_col)?;
    let c = validate_identifier(child_col)?;
    Ok(format!("{parent_alias}.{p} = {child_alias}.{c}"))
}

/// Build the `json_build_object('k', alias.col, ...)` fragment for a resolved
/// embed's scalar columns plus its nested embeds.
fn json_object_sql(
    re: &ResolvedEmbed,
    start_index: usize,
) -> Result<(String, Vec<QueryParam>, usize), EmbedError> {
    let mut pairs: Vec<String> = Vec::new();
    for col in &re.columns {
        let expr = parse_column_ref(&col.col_ref)?.to_sql();
        let key_lit = col.key.replace('\'', "''");
        pairs.push(format!("'{key_lit}', {}.{expr}", re.child_alias));
    }
    let mut params = Vec::new();
    let mut idx = start_index;
    for child in &re.children {
        let (item, mut p, next) = render_embed_item(child, idx)?;
        // `render_embed_item` yields `<sql> AS <name>`; extract both for the pair.
        let name = embed_output_name(&child.request);
        let expr = strip_as_alias(&item, name);
        let key_lit = name.replace('\'', "''");
        pairs.push(format!("'{key_lit}', {expr}"));
        params.append(&mut p);
        idx = next;
    }
    Ok((
        format!("json_build_object({})", pairs.join(", ")),
        params,
        idx,
    ))
}

/// Strip a trailing ` AS <name>` that `render_embed_item` appends, leaving the
/// bare subselect expression for embedding inside a `json_build_object` pair.
fn strip_as_alias(item: &str, name: &str) -> String {
    let suffix = format!(" AS {name}");
    item.strip_suffix(&suffix)
        .map_or_else(|| item.to_owned(), ToOwned::to_owned)
}

/// Render the child's `WHERE corr [AND filters]`, threading params.
fn child_where(
    re: &ResolvedEmbed,
    start_index: usize,
) -> Result<(String, Vec<QueryParam>, usize), EmbedError> {
    let corr = correlation_sql(
        &re.edge,
        &re.parent_alias,
        &re.child_alias,
        &re.request.target,
    )?;
    let (flt_sql, params, next) = render_child_filter(re, start_index)?;
    if flt_sql.is_empty() {
        Ok((corr, params, next))
    } else {
        Ok((format!("{corr} AND {flt_sql}"), params, next))
    }
}

/// Render the embed's routed filter, qualifying leaf columns with the child
/// alias. Empty when there are no routed filters.
fn render_child_filter(
    re: &ResolvedEmbed,
    start_index: usize,
) -> Result<(String, Vec<QueryParam>, usize), EmbedError> {
    let qualified = qualify_filter(&re.request.filter, &re.child_alias);
    if is_empty_filter(&qualified) {
        return Ok((String::new(), vec![], start_index));
    }
    let hints = re.cast_hints.qualified(&re.child_alias);
    let (sql, params, next) = qualified.render_with_hints(start_index, &hints)?;
    Ok((sql, params, next))
}

/// True for an `And([])` (the default empty filter).
fn is_empty_filter(f: &FilterTree) -> bool {
    matches!(f, FilterTree::And(children) if children.is_empty())
}

/// Rewrite every leaf column `col` → `<alias>.col`, preserving structure. The
/// column text is re-validated by `parse_column_ref` at render time, so this
/// only prepends a validated alias and a dot.
fn qualify_filter(f: &FilterTree, alias: &str) -> FilterTree {
    match f {
        FilterTree::Leaf {
            column,
            op,
            value,
            negate,
            quantifier,
            fts_config,
        } => FilterTree::Leaf {
            column: format!("{alias}.{column}"),
            op: *op,
            value: value.clone(),
            negate: *negate,
            quantifier: *quantifier,
            fts_config: fts_config.clone(),
        },
        FilterTree::And(children) => {
            FilterTree::And(children.iter().map(|c| qualify_filter(c, alias)).collect())
        }
        FilterTree::Or(children) => {
            FilterTree::Or(children.iter().map(|c| qualify_filter(c, alias)).collect())
        }
        FilterTree::Not(inner) => FilterTree::Not(Box::new(qualify_filter(inner, alias))),
    }
}

/// Render the embed's `ORDER BY` terms (without the `ORDER BY` keyword),
/// qualified with the child alias, for use inside `json_agg(... ORDER BY ...)`.
fn embed_order_terms(re: &ResolvedEmbed) -> String {
    if re.request.order.is_empty() {
        return String::new();
    }
    // `Order::to_sql()` yields `ORDER BY <terms>`; strip the keyword. Column
    // expressions are already validated; we qualify the leading identifier.
    let sql = re.request.order.to_sql();
    let terms = sql.strip_prefix("ORDER BY ").unwrap_or(&sql);
    // Qualify each comma-separated term's leading column with the child alias.
    terms
        .split(", ")
        .map(|t| format!("{}.{t}", re.child_alias))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Render one resolved embed to a SELECT-list item `(<subselect>) AS <name>`.
///
/// to-many → `COALESCE((SELECT json_agg(...) ...), '[]'::json)`; to-one →
/// `(SELECT json_build_object(...) ... LIMIT 1)`. Params thread from
/// `start_index`.
///
/// # Errors
/// Propagates [`EmbedError`] from identifier/filter rendering.
fn render_embed_item(
    re: &ResolvedEmbed,
    start_index: usize,
) -> Result<(String, Vec<QueryParam>, usize), EmbedError> {
    let name = embed_output_name(&re.request).to_owned();
    let (obj_sql, obj_params, idx) = json_object_sql(re, start_index)?;
    let (where_sql, mut where_params, next) = child_where(re, idx)?;
    let mut params = obj_params;
    params.append(&mut where_params);

    let child = &re.request.target;
    let alias = &re.child_alias;

    let sql = match re.edge.cardinality {
        Cardinality::ToOne => {
            format!("(SELECT {obj_sql} FROM {child} {alias} WHERE {where_sql} LIMIT 1) AS {name}")
        }
        Cardinality::ToMany => {
            let order_terms = embed_order_terms(re);
            let has_page = re.request.limit.is_some() || re.request.offset.is_some();
            if has_page {
                use std::fmt::Write as _;
                // LIMIT/OFFSET must live in a derived table; json_agg aggregates it.
                let mut derived = format!("SELECT * FROM {child} {alias} WHERE {where_sql}");
                if !order_terms.is_empty() {
                    let _ = write!(derived, " ORDER BY {order_terms}");
                }
                if let Some(l) = re.request.limit {
                    let _ = write!(derived, " LIMIT {l}");
                }
                if let Some(o) = re.request.offset {
                    let _ = write!(derived, " OFFSET {o}");
                }
                // Rebuild json object over the derived-table alias `sub`.
                let sub_obj = rekey_object(&obj_sql, alias, "sub");
                format!(
                    "COALESCE((SELECT json_agg({sub_obj}) FROM ({derived}) sub), '[]'::json) AS {name}"
                )
            } else {
                let agg = if order_terms.is_empty() {
                    format!("json_agg({obj_sql})")
                } else {
                    format!("json_agg({obj_sql} ORDER BY {order_terms})")
                };
                format!(
                    "COALESCE((SELECT {agg} FROM {child} {alias} WHERE {where_sql}), '[]'::json) AS {name}"
                )
            }
        }
    };
    Ok((sql, params, next))
}

/// Rewrite `<old_alias>.` occurrences to `<new_alias>.` inside a rendered
/// `json_build_object` (used when wrapping in a derived table).
fn rekey_object(obj_sql: &str, old_alias: &str, new_alias: &str) -> String {
    obj_sql.replace(&format!("{old_alias}."), &format!("{new_alias}."))
}

/// Render spread embed columns as flattened scalar subselect items lifted into
/// the parent projection: `(SELECT c.col FROM child c WHERE corr LIMIT 1) AS key`.
///
/// # Errors
/// Propagates [`EmbedError`]; the caller has already checked to-one cardinality.
fn render_spread_items(
    re: &ResolvedEmbed,
    start_index: usize,
) -> Result<(Vec<String>, Vec<QueryParam>, usize), EmbedError> {
    let mut items = Vec::with_capacity(re.columns.len());
    let mut params = Vec::new();
    let mut idx = start_index;
    for col in &re.columns {
        let (where_sql, mut p, next) = child_where(re, idx)?;
        let expr = parse_column_ref(&col.col_ref)?.to_sql();
        let key = validate_identifier(&col.key).map_err(|_| IdentError::Unsafe(col.key.clone()))?;
        items.push(format!(
            "(SELECT {}.{expr} FROM {} {} WHERE {where_sql} LIMIT 1) AS {key}",
            re.child_alias, re.request.target, re.child_alias
        ));
        params.append(&mut p);
        idx = next;
    }
    Ok((items, params, idx))
}

/// Build the full parent projection list: plain [`Select`] columns ++ embed JSON
/// items ++ spread items, as one comma-joined SQL string, threading indices.
///
/// `base_select` renders the parent's own columns (its `*` becomes
/// `<parent_alias>.*` is the caller's concern; here we emit it verbatim).
///
/// # Errors
/// Propagates [`EmbedError`] from embed rendering.
pub fn render_projection(
    base_select: &Select,
    resolved: &[ResolvedEmbed],
    start_index: usize,
) -> Result<(String, Vec<QueryParam>, usize), EmbedError> {
    let mut items: Vec<String> = vec![base_select.to_sql()];
    let mut params = Vec::new();
    let mut idx = start_index;
    for re in resolved {
        if re.request.spread {
            let (mut spread_items, mut p, next) = render_spread_items(re, idx)?;
            items.append(&mut spread_items);
            params.append(&mut p);
            idx = next;
        } else {
            let (item, mut p, next) = render_embed_item(re, idx)?;
            items.push(item);
            params.append(&mut p);
            idx = next;
        }
    }
    Ok((items.join(", "), params, idx))
}

/// Render the top-level `EXISTS (SELECT 1 FROM child ... )` guard for an
/// `!inner` embed (or a top-level filter-by-embed). ANDed into the parent WHERE.
///
/// # Errors
/// Propagates [`EmbedError`] from filter rendering.
fn render_inner_exists(
    re: &ResolvedEmbed,
    start_index: usize,
) -> Result<(String, Vec<QueryParam>, usize), EmbedError> {
    let (where_sql, params, next) = child_where(re, start_index)?;
    let sql = format!(
        "EXISTS (SELECT 1 FROM {} {} WHERE {where_sql})",
        re.request.target, re.child_alias
    );
    Ok((sql, params, next))
}

/// Collect the top-level `EXISTS` predicates contributed by `!inner` embeds,
/// threading params from `start_index`.
///
/// # Errors
/// Propagates [`EmbedError`].
pub fn render_inner_guards(
    resolved: &[ResolvedEmbed],
    start_index: usize,
) -> Result<(Vec<String>, Vec<QueryParam>, usize), EmbedError> {
    let mut preds = Vec::new();
    let mut params = Vec::new();
    let mut idx = start_index;
    for re in resolved {
        if re.request.join == JoinKind::Inner {
            let (sql, mut p, next) = render_inner_exists(re, idx)?;
            preds.push(sql);
            params.append(&mut p);
            idx = next;
        }
    }
    Ok((preds, params, idx))
}
