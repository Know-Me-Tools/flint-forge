//! Write plans: INSERT (bulk + upsert), UPDATE (PATCH), DELETE.
//!
//! Values are always bound as `$n`; column and relation identifiers are validated.
//! `Prefer` directives (`return=`, `resolution=`, `missing=`) are parsed into typed
//! options that shape the rendered statement.

use crate::filter::{FilterError, FilterTree};
use crate::ident::validate_identifier;
use crate::param::QueryParam;
use crate::plan::ParseError;

/// `Prefer: return=` — whether the write returns the affected rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ReturnKind {
    /// `return=minimal` — no body (default for PostgREST writes without the header).
    #[default]
    Minimal,
    /// `return=representation` — return the affected rows (`RETURNING *`).
    Representation,
}

/// `Prefer: resolution=` — insert conflict handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Resolution {
    /// No upsert; a conflict errors.
    #[default]
    None,
    /// `resolution=merge-duplicates` → `ON CONFLICT (...) DO UPDATE`.
    MergeDuplicates,
    /// `resolution=ignore-duplicates` → `ON CONFLICT (...) DO NOTHING`.
    IgnoreDuplicates,
}

/// Options controlling INSERT rendering, parsed from headers/query.
#[derive(Debug, Clone, Default)]
pub struct InsertOptions {
    pub returning: ReturnKind,
    pub resolution: Resolution,
    /// Conflict-target columns for upsert (`on_conflict=col1,col2`). Validated.
    pub on_conflict: Vec<String>,
    /// `Prefer: missing=default` — omitted columns take their DEFAULT rather than NULL.
    pub missing_default: bool,
}

/// A bulk INSERT plan. All rows must share the same column set (PostgREST rule).
#[derive(Debug, Clone)]
pub struct InsertPlan {
    pub relation: String,
    pub columns: Vec<String>,
    /// Row-major values; each inner vec has `columns.len()` entries.
    pub rows: Vec<Vec<QueryParam>>,
    pub options: InsertOptions,
}

impl InsertPlan {
    /// Build a validated bulk-insert plan.
    ///
    /// # Errors
    /// Returns [`ParseError`] on an unsafe relation/column, empty row set, or a
    /// row whose arity does not match `columns`.
    pub fn new(
        relation: &str,
        columns: Vec<String>,
        rows: Vec<Vec<QueryParam>>,
        options: InsertOptions,
    ) -> Result<Self, ParseError> {
        let relation = validate_identifier(relation)
            .map_err(|_| ParseError::UnsafeRelation(relation.to_owned()))?
            .to_owned();
        for c in &columns {
            validate_identifier(c).map_err(|_| ParseError::Ident(crate::IdentError::Unsafe(c.clone())))?;
        }
        for oc in &options.on_conflict {
            validate_identifier(oc)
                .map_err(|_| ParseError::Ident(crate::IdentError::Unsafe(oc.clone())))?;
        }
        if rows.is_empty() {
            return Err(ParseError::MalformedFilter("insert: empty row set".into()));
        }
        if let Some(bad) = rows.iter().find(|r| r.len() != columns.len()) {
            return Err(ParseError::MalformedFilter(format!(
                "insert: row arity {} != column count {}",
                bad.len(),
                columns.len()
            )));
        }
        Ok(Self {
            relation,
            columns,
            rows,
            options,
        })
    }

    /// Render the INSERT to `(sql, flattened params)`.
    #[must_use]
    pub fn render(&self) -> (String, Vec<QueryParam>) {
        let cols = self.columns.join(", ");
        let mut params = Vec::with_capacity(self.rows.len() * self.columns.len());
        let mut idx = 1usize;
        let mut tuples = Vec::with_capacity(self.rows.len());
        for row in &self.rows {
            let placeholders: Vec<String> = row
                .iter()
                .map(|p| {
                    let ph = format!("${idx}");
                    params.push(p.clone());
                    idx += 1;
                    ph
                })
                .collect();
            tuples.push(format!("({})", placeholders.join(", ")));
        }
        let mut sql = format!(
            "INSERT INTO {} ({cols}) VALUES {}",
            self.relation,
            tuples.join(", ")
        );
        match self.options.resolution {
            Resolution::None => {}
            Resolution::IgnoreDuplicates => {
                sql.push_str(&on_conflict_target(&self.options.on_conflict));
                sql.push_str(" DO NOTHING");
            }
            Resolution::MergeDuplicates => {
                sql.push_str(&on_conflict_target(&self.options.on_conflict));
                let sets: Vec<String> = self
                    .columns
                    .iter()
                    .map(|c| format!("{c} = EXCLUDED.{c}"))
                    .collect();
                sql.push_str(" DO UPDATE SET ");
                sql.push_str(&sets.join(", "));
            }
        }
        if self.options.returning == ReturnKind::Representation {
            sql.push_str(" RETURNING *");
        }
        (sql, params)
    }
}

fn on_conflict_target(cols: &[String]) -> String {
    if cols.is_empty() {
        " ON CONFLICT".to_owned()
    } else {
        format!(" ON CONFLICT ({})", cols.join(", "))
    }
}

/// An UPDATE (PATCH) plan: `SET col=$n,...` for rows matching `filter`.
#[derive(Debug, Clone)]
pub struct UpdatePlan {
    pub relation: String,
    pub assignments: Vec<(String, QueryParam)>,
    pub filter: FilterTree,
    pub returning: ReturnKind,
}

impl UpdatePlan {
    /// Render the UPDATE to `(sql, params)`. SET params come first, then WHERE params.
    ///
    /// # Errors
    /// Propagates [`FilterError`] from the WHERE tree.
    pub fn render(&self) -> Result<(String, Vec<QueryParam>), FilterError> {
        let mut params = Vec::new();
        let mut idx = 1usize;
        let sets: Vec<String> = self
            .assignments
            .iter()
            .map(|(col, val)| {
                let s = format!("{col} = ${idx}");
                params.push(val.clone());
                idx += 1;
                s
            })
            .collect();
        let (where_sql, mut where_params, _) = self.filter.render(idx)?;
        params.append(&mut where_params);
        let mut sql = format!("UPDATE {} SET {}", self.relation, sets.join(", "));
        if where_sql != "TRUE" {
            sql.push_str(" WHERE ");
            sql.push_str(&where_sql);
        }
        if self.returning == ReturnKind::Representation {
            sql.push_str(" RETURNING *");
        }
        Ok((sql, params))
    }
}

/// A DELETE plan for rows matching `filter`.
#[derive(Debug, Clone)]
pub struct DeletePlan {
    pub relation: String,
    pub filter: FilterTree,
    pub returning: ReturnKind,
}

impl DeletePlan {
    /// Render the DELETE to `(sql, params)`.
    ///
    /// # Errors
    /// Propagates [`FilterError`] from the WHERE tree.
    pub fn render(&self) -> Result<(String, Vec<QueryParam>), FilterError> {
        let (where_sql, params, _) = self.filter.render(1)?;
        let mut sql = format!("DELETE FROM {}", self.relation);
        if where_sql != "TRUE" {
            sql.push_str(" WHERE ");
            sql.push_str(&where_sql);
        }
        if self.returning == ReturnKind::Representation {
            sql.push_str(" RETURNING *");
        }
        Ok((sql, params))
    }
}

/// Parse a `Prefer` header into the write directives it carries.
#[must_use]
pub fn parse_write_prefer(prefer: &str) -> (ReturnKind, Resolution, bool) {
    let mut returning = ReturnKind::Minimal;
    let mut resolution = Resolution::None;
    let mut missing_default = false;
    for part in prefer.split([',', ' ']) {
        match part.trim() {
            "return=representation" => returning = ReturnKind::Representation,
            "return=minimal" => returning = ReturnKind::Minimal,
            "resolution=merge-duplicates" => resolution = Resolution::MergeDuplicates,
            "resolution=ignore-duplicates" => resolution = Resolution::IgnoreDuplicates,
            "missing=default" => missing_default = true,
            _ => {}
        }
    }
    (returning, resolution, missing_default)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn txt(s: &str) -> QueryParam {
        QueryParam::Text(s.into())
    }

    #[test]
    fn bulk_insert_multiple_rows() {
        let plan = InsertPlan::new(
            "users",
            vec!["name".into(), "email".into()],
            vec![
                vec![txt("a"), txt("a@x.com")],
                vec![txt("b"), txt("b@x.com")],
            ],
            InsertOptions::default(),
        )
        .unwrap();
        let (sql, params) = plan.render();
        assert_eq!(
            sql,
            "INSERT INTO users (name, email) VALUES ($1, $2), ($3, $4)"
        );
        assert_eq!(params.len(), 4);
    }

    #[test]
    fn insert_representation_returns_rows() {
        let plan = InsertPlan::new(
            "t",
            vec!["a".into()],
            vec![vec![txt("1")]],
            InsertOptions {
                returning: ReturnKind::Representation,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(plan.render().0.ends_with("RETURNING *"));
    }

    #[test]
    fn upsert_merge_duplicates() {
        let plan = InsertPlan::new(
            "t",
            vec!["id".into(), "v".into()],
            vec![vec![txt("1"), txt("x")]],
            InsertOptions {
                resolution: Resolution::MergeDuplicates,
                on_conflict: vec!["id".into()],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            plan.render().0,
            "INSERT INTO t (id, v) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET id = EXCLUDED.id, v = EXCLUDED.v"
        );
    }

    #[test]
    fn upsert_ignore_duplicates() {
        let plan = InsertPlan::new(
            "t",
            vec!["id".into()],
            vec![vec![txt("1")]],
            InsertOptions {
                resolution: Resolution::IgnoreDuplicates,
                on_conflict: vec!["id".into()],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            plan.render().0,
            "INSERT INTO t (id) VALUES ($1) ON CONFLICT (id) DO NOTHING"
        );
    }

    #[test]
    fn insert_rejects_arity_mismatch() {
        let err = InsertPlan::new(
            "t",
            vec!["a".into(), "b".into()],
            vec![vec![txt("1")]],
            InsertOptions::default(),
        )
        .unwrap_err();
        assert!(matches!(err, ParseError::MalformedFilter(_)));
    }

    #[test]
    fn insert_rejects_empty_rows_and_unsafe_ident() {
        assert!(InsertPlan::new("t", vec!["a".into()], vec![], InsertOptions::default()).is_err());
        assert!(matches!(
            InsertPlan::new("t; DROP", vec!["a".into()], vec![vec![txt("1")]], InsertOptions::default())
                .unwrap_err(),
            ParseError::UnsafeRelation(_)
        ));
        assert!(InsertPlan::new("t", vec!["a; DROP".into()], vec![vec![txt("1")]], InsertOptions::default()).is_err());
    }

    #[test]
    fn update_set_then_where_param_order() {
        let plan = UpdatePlan {
            relation: "t".into(),
            assignments: vec![("status".into(), txt("done"))],
            filter: FilterTree::Leaf {
                column: "id".into(),
                op: crate::Operator::Eq,
                value: "5".into(),
                negate: false,
                quantifier: None,
            },
            returning: ReturnKind::Representation,
        };
        let (sql, params) = plan.render().unwrap();
        assert_eq!(sql, "UPDATE t SET status = $1 WHERE id = $2 RETURNING *");
        assert_eq!(params, vec![txt("done"), txt("5")]);
    }

    #[test]
    fn delete_with_filter() {
        let plan = DeletePlan {
            relation: "t".into(),
            filter: FilterTree::Leaf {
                column: "id".into(),
                op: crate::Operator::Eq,
                value: "9".into(),
                negate: false,
                quantifier: None,
            },
            returning: ReturnKind::Minimal,
        };
        assert_eq!(plan.render().unwrap().0, "DELETE FROM t WHERE id = $1");
    }

    #[test]
    fn write_prefer_parse() {
        let (returning, resolution, missing) =
            parse_write_prefer("return=representation, resolution=merge-duplicates, missing=default");
        assert_eq!(returning, ReturnKind::Representation);
        assert_eq!(resolution, Resolution::MergeDuplicates);
        assert!(missing);
    }
}
