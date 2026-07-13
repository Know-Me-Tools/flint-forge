//! Column-type cast hints for parameter placeholders.
//!
//! `fdb-query` binds every user value as opaque text (see [`crate::param`]) so it
//! stays free of any database dependency. When a Postgres driver declares a bound
//! `String`'s type explicitly (as `sqlx` does), comparing it against a non-text
//! column (`id = $1` where `id` is `int4`) fails server-side: `operator does not
//! exist: integer = text`. The fix is an explicit cast on the *placeholder*,
//! `id = $1::int4` — Postgres then applies `int4`'s text-input parser to the same
//! bytes, matching the target column's type without touching its index usage.
//!
//! [`CastHints`] is a caller-supplied, column-name → Postgres base-type lookup.
//! `fdb-query` stays schema-agnostic: it only renders a cast when the caller
//! supplies one. Every render entry point defaults to an empty [`CastHints`],
//! so existing (schema-free) callers and tests are unaffected.

use std::collections::HashMap;

use crate::ident::validate_identifier;

/// Postgres base types whose text representation already matches what a bound
/// `String`/text parameter carries — no cast is needed (and casting `text` to
/// `text`/`varchar`/etc. would just be noise).
fn is_text_compatible(base_type: &str) -> bool {
    matches!(
        base_type.to_ascii_lowercase().as_str(),
        "text" | "varchar" | "bpchar" | "name" | "citext"
    )
}

/// Strip a trailing parameterized modifier (`numeric(10,2)` → `numeric`,
/// `varchar(255)` → `varchar`), then map a handful of multi-word SQL-standard
/// type names — as produced by `format_type()`/this crate's own normalization
/// pass (`int8` canonicalizes to `bigint`, but `float8` canonicalizes to the
/// two-word `double precision`) — to their single-word Postgres alias. A
/// two-word name would otherwise fail identifier validation and silently drop
/// the cast, leaving that column's exact class of bug unfixed.
fn base_type_name(pg_type: &str) -> String {
    let trimmed = pg_type.split('(').next().unwrap_or(pg_type).trim();
    let mapped = match trimmed.to_ascii_lowercase().as_str() {
        "double precision" => "float8",
        "timestamp without time zone" => "timestamp",
        "timestamp with time zone" => "timestamptz",
        "time without time zone" => "time",
        "time with time zone" => "timetz",
        "character varying" => "varchar",
        "bit varying" => "varbit",
        _ => trimmed,
    };
    mapped.to_owned()
}

/// A validated column-name → Postgres base-type lookup for cast rendering.
///
/// Built by the caller (`fdb-reflection`) from its `DatabaseModel`, never by
/// `fdb-query` itself. Entries that are text-compatible or fail identifier
/// validation are silently dropped at construction — defense-in-depth, since a
/// resolved cast type is spliced directly into SQL (never a bind parameter).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CastHints(HashMap<String, String>);

impl CastHints {
    /// An empty hint set — every leaf renders with no cast (today's behavior).
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// Build from `(column_name, pg_type)` pairs, e.g. sourced from
    /// `DatabaseModel`'s `Column { name, pg_type, .. }` list.
    ///
    /// Text-compatible types and unsafe type names are dropped, not errored —
    /// this is a best-effort optimization hint, not a validated schema contract.
    #[must_use]
    pub fn from_pairs<I, S1, S2>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (S1, S2)>,
        S1: Into<String>,
        S2: AsRef<str>,
    {
        let mut map = HashMap::new();
        for (col, pg_type) in pairs {
            let base = base_type_name(pg_type.as_ref());
            if is_text_compatible(&base) {
                continue;
            }
            if validate_identifier(&base).is_ok() {
                map.insert(col.into(), base);
            }
        }
        Self(map)
    }

    /// The resolved cast target for `column`, if any.
    #[must_use]
    pub fn get(&self, column: &str) -> Option<&str> {
        self.0.get(column).map(String::as_str)
    }

    /// True when no column has a cast hint.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Re-key every entry under `"{prefix}.{column}"`, for qualifying a child
    /// table's hints with its correlation alias inside an embed (e.g. `orders`'s
    /// `total` hint becomes `orders_1.total` after `qualify_filter` renames the
    /// leaf column to the child alias).
    #[must_use]
    pub fn qualified(&self, prefix: &str) -> Self {
        Self(
            self.0
                .iter()
                .map(|(col, ty)| (format!("{prefix}.{col}"), ty.clone()))
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_compatible_types_are_dropped() {
        let hints = CastHints::from_pairs([("name", "text"), ("email", "varchar(255)")]);
        assert!(hints.is_empty());
    }

    #[test]
    fn scalar_types_are_kept_with_modifiers_stripped() {
        let hints = CastHints::from_pairs([
            ("id", "int4"),
            ("total", "numeric(10,2)"),
            ("active", "bool"),
        ]);
        assert_eq!(hints.get("id"), Some("int4"));
        assert_eq!(hints.get("total"), Some("numeric"));
        assert_eq!(hints.get("active"), Some("bool"));
    }

    #[test]
    fn multiword_sql_standard_names_map_to_single_word_aliases() {
        // `float8` canonicalizes to the two-word `double precision` in this
        // crate's own normalization pass; a bare identifier check would drop it.
        let hints = CastHints::from_pairs([
            ("price", "double precision"),
            ("ts", "timestamp with time zone"),
        ]);
        assert_eq!(hints.get("price"), Some("float8"));
        assert_eq!(hints.get("ts"), Some("timestamptz"));
    }

    #[test]
    fn unsafe_type_name_is_dropped_defensively() {
        let hints = CastHints::from_pairs([("id", "int4; DROP TABLE users--")]);
        assert_eq!(hints.get("id"), None);
    }

    #[test]
    fn qualified_reprefixes_every_key() {
        let hints = CastHints::from_pairs([("total", "int4")]);
        let q = hints.qualified("orders_1");
        assert_eq!(q.get("orders_1.total"), Some("int4"));
        assert_eq!(q.get("total"), None);
    }
}
