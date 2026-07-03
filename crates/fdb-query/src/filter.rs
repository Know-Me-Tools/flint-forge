//! Filter trees — the parsed WHERE clause.
//!
//! A PostgREST request expresses row filters as either simple `column=op.value`
//! pairs (implicitly AND-ed) or explicit logical groups: `and=(...)`, `or=(...)`,
//! and their negations `not.and=(...)`, `not.or=(...)`, nested to any depth. This
//! module turns those into a [`FilterTree`] and renders it to a parameterized SQL
//! predicate, reusing [`crate::operator::render_condition`] for leaves.

use crate::fts::FtsConfig;
use crate::ident::{IdentError, parse_column_ref};
use crate::operator::{Operator, Quantifier, RenderError, render_condition};
use crate::param::QueryParam;

/// A node in the filter tree.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterTree {
    /// A leaf condition: `<column-ref> <op> <value>`, optionally negated / quantified.
    Leaf {
        /// Raw column reference (validated at render time via `parse_column_ref`).
        column: String,
        op: Operator,
        value: String,
        negate: bool,
        quantifier: Option<Quantifier>,
        /// Text-search config for the four FTS operators; `None` otherwise. The
        /// structural analog of `quantifier`, threaded through to `render_fts`.
        fts_config: Option<FtsConfig>,
    },
    /// `a AND b AND ...`
    And(Vec<FilterTree>),
    /// `a OR b OR ...`
    Or(Vec<FilterTree>),
    /// `NOT (inner)` — the negation of a whole group (`not.and` / `not.or`).
    Not(Box<FilterTree>),
}

impl FilterTree {
    /// Render the tree to a SQL predicate plus ordered bind params, numbering
    /// placeholders from `start_index`.
    ///
    /// Returns `(sql, params, next_index)`. An empty `And`/`Or` renders to the
    /// identity for its connective (`TRUE` for AND, `FALSE` for OR), matching SQL
    /// fold semantics so callers never emit a dangling connective.
    ///
    /// # Errors
    /// Propagates [`FilterError`] from identifier validation or condition rendering.
    pub fn render(&self, start_index: usize) -> Result<(String, Vec<QueryParam>, usize), FilterError> {
        match self {
            Self::Leaf {
                column,
                op,
                value,
                negate,
                quantifier,
                fts_config,
            } => {
                let col_ref = parse_column_ref(column)?;
                let (sql, params, next) = render_condition(
                    &col_ref.to_sql(),
                    *op,
                    value,
                    *negate,
                    *quantifier,
                    fts_config.as_ref(),
                    start_index,
                )?;
                Ok((sql, params, next))
            }
            Self::And(children) => Self::render_group(children, "AND", "TRUE", start_index),
            Self::Or(children) => Self::render_group(children, "OR", "FALSE", start_index),
            Self::Not(inner) => {
                let (sql, params, next) = inner.render(start_index)?;
                Ok((format!("NOT ({sql})"), params, next))
            }
        }
    }

    fn render_group(
        children: &[FilterTree],
        connective: &str,
        identity: &str,
        start_index: usize,
    ) -> Result<(String, Vec<QueryParam>, usize), FilterError> {
        if children.is_empty() {
            return Ok((identity.to_owned(), vec![], start_index));
        }
        let mut parts = Vec::with_capacity(children.len());
        let mut params = Vec::new();
        let mut idx = start_index;
        for child in children {
            let (sql, mut p, next) = child.render(idx)?;
            parts.push(sql);
            params.append(&mut p);
            idx = next;
        }
        let joined = parts.join(&format!(" {connective} "));
        // Parenthesize multi-child groups so nesting precedence is explicit.
        let sql = if parts.len() > 1 {
            format!("({joined})")
        } else {
            joined
        };
        Ok((sql, params, idx))
    }
}

/// Errors from filter-tree construction/rendering.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FilterError {
    /// A column reference failed identifier validation.
    #[error(transparent)]
    Ident(#[from] IdentError),
    /// A condition failed to render (bad `is` operand, misapplied quantifier).
    #[error(transparent)]
    Render(#[from] RenderError),
    /// A logical group value was not the expected `(a.op.v,b.op.v,...)` form.
    #[error("malformed logical group: {0}")]
    MalformedGroup(String),
    /// A filter value was not the expected `op.value` form.
    #[error("malformed filter for `{0}`: expected `<op>.<value>`")]
    MalformedFilter(String),
    /// The operator token was not recognized.
    #[error("unknown operator: {0}")]
    UnknownOp(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(col: &str, op: Operator, val: &str) -> FilterTree {
        FilterTree::Leaf {
            column: col.into(),
            op,
            value: val.into(),
            negate: false,
            quantifier: None,
            fts_config: None,
        }
    }

    #[test]
    fn single_leaf_renders_bare() {
        let (sql, params, next) = leaf("age", Operator::Gte, "18").render(1).unwrap();
        assert_eq!(sql, "age >= $1");
        assert_eq!(params, vec![QueryParam::Text("18".into())]);
        assert_eq!(next, 2);
    }

    #[test]
    fn and_group_joins_and_parenthesizes() {
        let tree = FilterTree::And(vec![
            leaf("age", Operator::Gte, "18"),
            leaf("status", Operator::Eq, "active"),
        ]);
        let (sql, params, next) = tree.render(1).unwrap();
        assert_eq!(sql, "(age >= $1 AND status = $2)");
        assert_eq!(
            params,
            vec![QueryParam::Text("18".into()), QueryParam::Text("active".into())]
        );
        assert_eq!(next, 3);
    }

    #[test]
    fn or_group_uses_or_connective() {
        let tree = FilterTree::Or(vec![
            leaf("a", Operator::Eq, "1"),
            leaf("b", Operator::Eq, "2"),
        ]);
        assert_eq!(tree.render(1).unwrap().0, "(a = $1 OR b = $2)");
    }

    #[test]
    fn nested_and_or_preserves_precedence_and_index() {
        // and(a.gt.1, or(b.eq.2, c.eq.3))
        let tree = FilterTree::And(vec![
            leaf("a", Operator::Gt, "1"),
            FilterTree::Or(vec![leaf("b", Operator::Eq, "2"), leaf("c", Operator::Eq, "3")]),
        ]);
        let (sql, params, next) = tree.render(1).unwrap();
        assert_eq!(sql, "(a > $1 AND (b = $2 OR c = $3))");
        assert_eq!(params.len(), 3);
        assert_eq!(next, 4);
    }

    #[test]
    fn not_group_wraps() {
        let tree = FilterTree::Not(Box::new(FilterTree::And(vec![
            leaf("a", Operator::Gte, "0"),
            leaf("a", Operator::Lte, "100"),
        ])));
        assert_eq!(tree.render(1).unwrap().0, "NOT ((a >= $1 AND a <= $2))");
    }

    #[test]
    fn empty_groups_render_identity() {
        assert_eq!(FilterTree::And(vec![]).render(1).unwrap().0, "TRUE");
        assert_eq!(FilterTree::Or(vec![]).render(1).unwrap().0, "FALSE");
    }

    #[test]
    fn leaf_with_json_path_and_negation() {
        let tree = FilterTree::Leaf {
            column: "data->>role".into(),
            op: Operator::Eq,
            value: "admin".into(),
            negate: true,
            quantifier: None,
            fts_config: None,
        };
        assert_eq!(tree.render(1).unwrap().0, "NOT (data ->> 'role' = $1)");
    }

    #[test]
    fn unsafe_column_in_leaf_errors() {
        let tree = leaf("x; DROP", Operator::Eq, "1");
        assert!(matches!(tree.render(1).unwrap_err(), FilterError::Ident(_)));
    }

    #[test]
    fn fts_leaf_threads_config_to_render() {
        let tree = FilterTree::Leaf {
            column: "body".into(),
            op: Operator::Fts,
            value: "cat".into(),
            negate: false,
            quantifier: None,
            fts_config: Some(FtsConfig::parse("english").unwrap()),
        };
        let (sql, params, next) = tree.render(1).unwrap();
        assert_eq!(sql, "body @@ to_tsquery('english', $1)");
        assert_eq!(params, vec![QueryParam::Text("cat".into())]);
        assert_eq!(next, 2);
    }
}
