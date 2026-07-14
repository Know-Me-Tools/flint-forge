//! PostgREST horizontal-filtering operators.
//!
//! The full operator surface: comparison, pattern, membership, null, range, and
//! containment operators, plus the `not.` negation prefix and the `any()`/`all()`
//! modifiers. Rendering always binds user values as parameters; only the operator
//! keyword (a fixed string) and the already-validated column reference reach SQL
//! directly.

mod render;
#[cfg(test)]
mod tests;

pub use render::render_condition;

use crate::fts::FtsKind;

/// A PostgREST filter operator.
///
/// Token → operator mapping follows the PostgREST "Operators" reference. Range
/// operators (`sl`, `sr`, `nxr`, `nxl`, `adj`, `ov`) target range/array columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Operator {
    /// `eq` → `=`
    Eq,
    /// `neq` → `<>`
    Neq,
    /// `gt` → `>`
    Gt,
    /// `gte` → `>=`
    Gte,
    /// `lt` → `<`
    Lt,
    /// `lte` → `<=`
    Lte,
    /// `like` → `LIKE`
    Like,
    /// `ilike` → `ILIKE`
    Ilike,
    /// `match` → `~` (POSIX regex)
    Match,
    /// `imatch` → `~*` (case-insensitive POSIX regex)
    Imatch,
    /// `in` → `= ANY(...)`
    In,
    /// `is` → `IS` (null / true / false / unknown)
    Is,
    /// `isdistinct` → `IS DISTINCT FROM`
    IsDistinct,
    /// `cs` → `@>` (contains)
    Cs,
    /// `cd` → `<@` (contained by)
    Cd,
    /// `ov` → `&&` (overlap)
    Ov,
    /// `sl` → `<<` (strictly left of)
    Sl,
    /// `sr` → `>>` (strictly right of)
    Sr,
    /// `nxr` → `&<` (does not extend to the right of)
    Nxr,
    /// `nxl` → `&>` (does not extend to the left of)
    Nxl,
    /// `adj` → `-|-` (adjacent)
    Adj,
    /// `fts` → `@@ to_tsquery(...)` (full-text search)
    Fts,
    /// `plfts` → `@@ plainto_tsquery(...)` (plain full-text search)
    Plfts,
    /// `phfts` → `@@ phraseto_tsquery(...)` (phrase full-text search)
    Phfts,
    /// `wfts` → `@@ websearch_to_tsquery(...)` (web-search full-text search)
    Wfts,
}

/// The `any`/`all` modifier applied to a scalar comparison operator, e.g.
/// `?id=eq(any).{1,2,3}` → `id = ANY('{1,2,3}')`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Quantifier {
    /// `= ANY(array)` — true if the comparison holds for any element.
    Any,
    /// `= ALL(array)` — true only if the comparison holds for every element.
    All,
}

impl Operator {
    /// Parse the operator keyword (the token before the first `.` in a filter,
    /// after any `not.` prefix and any `(any)`/`(all)` modifier have been stripped).
    #[must_use]
    pub fn parse(token: &str) -> Option<Self> {
        Some(match token {
            "eq" => Self::Eq,
            "neq" => Self::Neq,
            "gt" => Self::Gt,
            "gte" => Self::Gte,
            "lt" => Self::Lt,
            "lte" => Self::Lte,
            "like" => Self::Like,
            "ilike" => Self::Ilike,
            "match" => Self::Match,
            "imatch" => Self::Imatch,
            "in" => Self::In,
            "is" => Self::Is,
            "isdistinct" => Self::IsDistinct,
            "cs" => Self::Cs,
            "cd" => Self::Cd,
            "ov" => Self::Ov,
            "sl" => Self::Sl,
            "sr" => Self::Sr,
            "nxr" => Self::Nxr,
            "nxl" => Self::Nxl,
            "adj" => Self::Adj,
            "fts" => Self::Fts,
            "plfts" => Self::Plfts,
            "phfts" => Self::Phfts,
            "wfts" => Self::Wfts,
            _ => return None,
        })
    }

    /// The SQL infix keyword for operators that render as `col <op> $n`.
    ///
    /// Returns `None` for operators with bespoke rendering (`In`, `Is`), which the
    /// renderer handles separately.
    #[must_use]
    pub fn sql_infix(self) -> Option<&'static str> {
        Some(match self {
            Self::Eq => "=",
            Self::Neq => "<>",
            Self::Gt => ">",
            Self::Gte => ">=",
            Self::Lt => "<",
            Self::Lte => "<=",
            Self::Like => "LIKE",
            Self::Ilike => "ILIKE",
            Self::Match => "~",
            Self::Imatch => "~*",
            Self::IsDistinct => "IS DISTINCT FROM",
            Self::Cs => "@>",
            Self::Cd => "<@",
            Self::Ov => "&&",
            Self::Sl => "<<",
            Self::Sr => ">>",
            Self::Nxr => "&<",
            Self::Nxl => "&>",
            Self::Adj => "-|-",
            Self::In | Self::Is | Self::Fts | Self::Plfts | Self::Phfts | Self::Wfts => {
                return None;
            }
        })
    }

    /// The full-text-search kind for this operator, if any.
    ///
    /// Returns `Some(..)` only for the four FTS operators; `None` otherwise. Used
    /// by [`render_condition`] to branch into [`crate::fts::render_fts`] and by the
    /// parser to decide the paren suffix is a text-search config rather than a
    /// quantifier.
    #[must_use]
    pub fn fts_kind(self) -> Option<FtsKind> {
        Some(match self {
            Self::Fts => FtsKind::Fts,
            Self::Plfts => FtsKind::Plfts,
            Self::Phfts => FtsKind::Phfts,
            Self::Wfts => FtsKind::Wfts,
            _ => return None,
        })
    }

    /// Whether the `any`/`all` quantifier modifier is valid for this operator.
    /// PostgREST allows it on the scalar comparison and pattern operators.
    #[must_use]
    pub fn allows_quantifier(self) -> bool {
        matches!(
            self,
            Self::Eq
                | Self::Neq
                | Self::Gt
                | Self::Gte
                | Self::Lt
                | Self::Lte
                | Self::Like
                | Self::Ilike
                | Self::Match
                | Self::Imatch
        )
    }

    /// Whether this operator's bound value should be encoded as a `jsonb` literal
    /// rather than plain text. Containment operators compare JSON/array structure.
    #[must_use]
    pub fn binds_json(self) -> bool {
        matches!(self, Self::Cs | Self::Cd)
    }
}

/// Errors produced while rendering a condition.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RenderError {
    /// The `is` operand was not one of null/true/false/unknown.
    #[error("invalid `is` value: {0} (expected null/true/false/unknown)")]
    InvalidIs(String),
    /// An `any`/`all` quantifier was applied to an operator that does not accept it.
    #[error("operator `{0}` does not accept an any/all quantifier")]
    QuantifierNotAllowed(&'static str),
    /// A full-text-search config name failed identifier validation.
    #[error("invalid text-search config: {0}")]
    InvalidFtsConfig(String),
}
