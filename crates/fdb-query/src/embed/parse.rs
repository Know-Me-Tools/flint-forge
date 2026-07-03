//! Embed-aware `select=` parsing and top-level embedded-param routing.
//!
//! Schema-free: produces an [`EmbedSelect`] tree of [`EmbedRequest`] nodes.
//! FK resolution happens later in `render::resolve_embeds`.

use crate::clause::Order;
use crate::filter::FilterTree;
use crate::ident::{IdentError, parse_column_ref, validate_identifier};

use super::schema::{
    EmbedError, EmbedRequest, EmbedSelect, JoinKind, ScalarCol, embed_output_name,
};


/// Parse an embed-aware `select=` value into scalar columns + embed requests.
///
/// Reuses top-level comma splitting and recurses into parenthesized embed
/// bodies. Classifies each token: bare column → scalar; `name(...)` → embed;
/// `...name(...)` → spread embed; `name!hint(...)` → `!fk` / `!inner` / `!left`;
/// `alias:name(...)` → aliased embed; `alias:col` → renamed scalar column.
///
/// # Errors
/// Returns [`EmbedError`] for unbalanced parens or unsafe identifiers.
pub fn parse_embed_select(raw: &str) -> Result<EmbedSelect, EmbedError> {
    let mut sel = EmbedSelect::default();
    for token in split_top_level_commas(raw) {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        if let Some(open) = find_top_level_open_paren(token) {
            sel.embeds.push(parse_embed_token(token, open)?);
        } else {
            sel.columns.push(parse_scalar_token(token)?);
        }
    }
    Ok(sel)
}

/// Parse a scalar `[alias:]column-ref` (or bare `*`) into a [`ScalarCol`].
fn parse_scalar_token(token: &str) -> Result<ScalarCol, EmbedError> {
    let (alias, col_part) = split_alias(token);
    if let Some(a) = alias {
        validate_identifier(a).map_err(|_| IdentError::Unsafe(a.to_owned()))?;
    }
    if col_part == "*" {
        return Ok(ScalarCol {
            key: "*".to_owned(),
            col_ref: "*".to_owned(),
            star: true,
        });
    }
    let cref = parse_column_ref(col_part)?;
    let key = alias.map_or_else(|| cref.base().to_owned(), ToOwned::to_owned);
    Ok(ScalarCol {
        key,
        col_ref: col_part.to_owned(),
        star: false,
    })
}

/// Parse an embed token `[...][alias:]name[!hint](body)` given the index of its
/// opening top-level paren.
fn parse_embed_token(token: &str, open: usize) -> Result<EmbedRequest, EmbedError> {
    if !token.ends_with(')') {
        return Err(EmbedError::MalformedEmbed(token.to_owned()));
    }
    let head = &token[..open];
    let body = &token[open + 1..token.len() - 1];

    let (spread, head) = match head.strip_prefix("...") {
        Some(rest) => (true, rest),
        None => (false, head),
    };
    let (alias, name_part) = split_alias(head);
    let alias = match alias {
        Some(a) => Some(
            validate_identifier(a)
                .map_err(|_| IdentError::Unsafe(a.to_owned()))?
                .to_owned(),
        ),
        None => None,
    };

    // Split off `!hint` (join kind or FK name).
    let (name, fk_hint, join) = match name_part.split_once('!') {
        Some((n, hint)) => match hint {
            "inner" => (n, None, JoinKind::Inner),
            "left" => (n, None, JoinKind::Left),
            fk => (n, Some(fk.to_owned()), JoinKind::Left),
        },
        None => (name_part, None, JoinKind::Left),
    };
    let name = validate_identifier(name)
        .map_err(|_| IdentError::Unsafe(name.to_owned()))?
        .to_owned();
    if let Some(fk) = &fk_hint {
        validate_identifier(fk).map_err(|_| IdentError::Unsafe(fk.clone()))?;
    }

    let select = parse_embed_select(body)?;
    Ok(EmbedRequest {
        alias,
        target: name,
        fk_hint,
        join,
        spread,
        select,
        filter: FilterTree::And(vec![]),
        order: Order::default(),
        limit: None,
        offset: None,
    })
}

// ---------------------------------------------------------------------------
// Param routing (dotted-prefix → embed node)
// ---------------------------------------------------------------------------

/// Route a top-level embedded param onto the matching [`EmbedRequest`] by dotted
/// alias/name prefix. Handles embedded filters (`child.col=eq.x`), embedded
/// order (key `order`, value `child.col.desc`), and embedded pagination
/// (`child.limit` / `child.offset`).
///
/// Returns `true` when the param was consumed by an embed, `false` when it is a
/// plain top-level column/clause the caller should handle itself.
///
/// # Errors
/// Returns [`EmbedError`] for a malformed embedded filter or an unknown modifier.
pub fn route_embedded_param(
    sel: &mut EmbedSelect,
    key: &str,
    value: &str,
) -> Result<bool, EmbedError> {
    // `order=child.col.dir` — dotted VALUE names the embed.
    if key == "order" {
        if let Some((head, rest)) = value.split_once('.') {
            if let Some(child) = find_embed_mut(sel, head) {
                route_embedded_param(&mut child.select, "order", rest)?;
                // Re-parse the child order from the remaining dotted terms.
                child.order = Order::parse(rest)?;
                return Ok(true);
            }
        }
        return Ok(false);
    }

    // `child.col=...`, `child.limit=`, `child.offset=` — dotted KEY names the embed.
    let Some((head, rest)) = key.split_once('.') else {
        return Ok(false);
    };
    let Some(child) = find_embed_mut(sel, head) else {
        return Ok(false);
    };

    match rest {
        "limit" => {
            child.limit = Some(
                value
                    .parse()
                    .map_err(|_| EmbedError::MalformedEmbed(format!("{head}.limit={value}")))?,
            );
        }
        "offset" => {
            child.offset = Some(
                value
                    .parse()
                    .map_err(|_| EmbedError::MalformedEmbed(format!("{head}.offset={value}")))?,
            );
        }
        col if col.contains('.') => {
            // Deeper nesting: `child.grand.col` — recurse.
            return route_embedded_param(&mut child.select, rest, value);
        }
        col => {
            let leaf = parse_embedded_leaf(col, value)?;
            push_leaf(&mut child.filter, leaf);
        }
    }
    Ok(true)
}

/// Find a direct child embed by its output name (alias or target).
fn find_embed_mut<'a>(sel: &'a mut EmbedSelect, name: &str) -> Option<&'a mut EmbedRequest> {
    sel.embeds
        .iter_mut()
        .find(|e| embed_output_name(e) == name)
}

/// Append a leaf into an embed's top-level `And` filter, creating one if needed.
fn push_leaf(filter: &mut FilterTree, leaf: FilterTree) {
    match filter {
        FilterTree::And(children) => children.push(leaf),
        other => {
            let existing = std::mem::replace(other, FilterTree::And(vec![]));
            if let FilterTree::And(children) = other {
                children.push(existing);
                children.push(leaf);
            }
        }
    }
}

/// Parse a `col=[not.]op[(any|all)].value` embedded filter into a leaf.
fn parse_embedded_leaf(column: &str, raw: &str) -> Result<FilterTree, EmbedError> {
    use crate::operator::{Operator, Quantifier};
    let (negate, rest) = match raw.strip_prefix("not.") {
        Some(r) => (true, r),
        None => (false, raw),
    };
    let (op_token, value) = rest
        .split_once('.')
        .ok_or_else(|| EmbedError::MalformedEmbed(format!("{column}={raw}")))?;
    let (op_name, quantifier) = if let Some(b) = op_token.strip_suffix("(any)") {
        (b, Some(Quantifier::Any))
    } else if let Some(b) = op_token.strip_suffix("(all)") {
        (b, Some(Quantifier::All))
    } else {
        (op_token, None)
    };
    let op = Operator::parse(op_name)
        .ok_or_else(|| EmbedError::MalformedEmbed(format!("unknown operator: {op_name}")))?;
    Ok(FilterTree::Leaf {
        column: column.to_owned(),
        op,
        value: value.to_owned(),
        negate,
        quantifier,
        fts_config: None,
    })
}

// ---------------------------------------------------------------------------
// Local tokenizer helpers (mirrors clause.rs, embed-aware)
// ---------------------------------------------------------------------------

/// Split on commas not nested inside parentheses. (Local copy so `embed.rs`
/// stays self-contained; identical semantics to `clause::split_top_level_commas`.)
fn split_top_level_commas(raw: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (i, c) in raw.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                out.push(&raw[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&raw[start..]);
    out
}

/// Index of the first top-level `(` in a token, or `None` if it has none.
fn find_top_level_open_paren(token: &str) -> Option<usize> {
    token.char_indices().find(|&(_, c)| c == '(').map(|(i, _)| i)
}

/// Split an `alias:rest` token on a single leading `:` that is not part of a
/// `::cast`. Returns `(Some(alias), rest)` or `(None, token)`.
fn split_alias(token: &str) -> (Option<&str>, &str) {
    let bytes = token.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b':' {
            if bytes.get(i + 1) == Some(&b':') {
                i += 2;
                continue;
            }
            return (Some(&token[..i]), &token[i + 1..]);
        }
        i += 1;
    }
    (None, token)
}
