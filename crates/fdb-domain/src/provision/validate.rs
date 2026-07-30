//! Spec validation: every identifier, every default expression, every
//! structural rule — before any SQL is generated (FFS-001 §8 task 1.2).
//!
//! Validation is a single pass that returns the *first* error (fail fast at
//! the boundary, per repo input-validation rules) plus, on success, the
//! warnings the plan must carry (e.g. acknowledged unscoped tables).

use forge_domain::is_safe_identifier;

use super::spec::{SchemaSpec, TableSpec};

/// Schemas the provisioning API refuses unconditionally, before any
/// operator allowlist is consulted (FFS-001 D4).
const RESERVED_EXACT: &[&str] = &["public", "information_schema"];
const RESERVED_PREFIXES: &[&str] = &["flint_", "pg_"];

/// The default expressions allowed verbatim beyond simple literals.
const ALLOWED_DEFAULT_FNS: &[&str] = &["now()", "gen_random_uuid()"];

/// A spec rejected by validation. Carries enough context to name the exact
/// offending element without echoing anything that failed identifier
/// validation back into logs verbatim beyond the value itself (values here
/// are caller-supplied names, never secrets).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SpecError {
    /// The namespace is not a safe, dot-free identifier.
    BadNamespace(String),
    /// The namespace is reserved and can never be provisioned.
    ReservedNamespace(String),
    /// The spec declares no tables.
    NoTables,
    /// A table declares no columns.
    NoColumns(String),
    /// An identifier (table, column, or index name) failed safety validation.
    BadIdentifier {
        /// What kind of identifier failed (`table`, `column`, `index`,
        /// `index column`).
        kind: &'static str,
        /// The offending value.
        value: String,
    },
    /// Two elements of the same kind share a name.
    Duplicate {
        /// What kind of element is duplicated.
        kind: &'static str,
        /// The duplicated name.
        value: String,
    },
    /// `tenant_id` was declared by the caller on a tenant-scoped table; the
    /// generator owns that column (FFS-001 D5).
    TenantIdReserved(String),
    /// `tenant_scoped: false` without `acknowledge_unscoped: true`.
    UnscopedNotAcknowledged(String),
    /// A default expression is neither an allowed literal nor an allowlisted
    /// function.
    BadDefault {
        /// The table and column, `table.column`.
        column: String,
        /// The rejected expression.
        value: String,
    },
    /// An index references a column not declared on its table.
    UnknownIndexColumn {
        /// The index name.
        index: String,
        /// The missing column.
        column: String,
    },
    /// A caller-declared index uses the name of the generated tenant index
    /// (`{table}_tenant_idx`); `IF NOT EXISTS` would then silently skip one
    /// of the two, so the collision is refused up front.
    ReservedIndexName(String),
}

impl std::fmt::Display for SpecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadNamespace(ns) => write!(f, "namespace `{ns}` is not a valid identifier"),
            Self::ReservedNamespace(ns) => write!(
                f,
                "namespace `{ns}` is reserved and can never be provisioned"
            ),
            Self::NoTables => f.write_str("spec declares no tables"),
            Self::NoColumns(t) => write!(f, "table `{t}` declares no columns"),
            Self::BadIdentifier { kind, value } => {
                write!(f, "{kind} name `{value}` is not a valid identifier")
            }
            Self::Duplicate { kind, value } => write!(f, "duplicate {kind} name `{value}`"),
            Self::TenantIdReserved(t) => write!(
                f,
                "table `{t}`: `tenant_id` is generated on tenant-scoped tables and cannot be declared by the caller"
            ),
            Self::UnscopedNotAcknowledged(t) => write!(
                f,
                "table `{t}`: tenantScoped=false requires acknowledgeUnscoped=true"
            ),
            Self::BadDefault { column, value } => write!(
                f,
                "column `{column}`: default `{value}` is not an allowed literal or function"
            ),
            Self::UnknownIndexColumn { index, column } => write!(
                f,
                "index `{index}` references undeclared column `{column}`"
            ),
            Self::ReservedIndexName(name) => write!(
                f,
                "index name `{name}` is reserved for the generated tenant index"
            ),
        }
    }
}

impl std::error::Error for SpecError {}

/// Validate a full spec. Returns the plan warnings on success.
///
/// # Errors
///
/// Returns the first [`SpecError`] encountered; see the enum variants for the
/// complete rule set.
pub fn validate_spec(spec: &SchemaSpec) -> Result<Vec<String>, SpecError> {
    let ns = spec.namespace.as_str();
    // Namespaces are single-segment: `is_safe_identifier` accepts dotted
    // names (schema.table), which would smuggle a second segment here.
    if ns.contains('.') || !is_safe_identifier(ns) {
        return Err(SpecError::BadNamespace(ns.to_owned()));
    }
    if RESERVED_EXACT.contains(&ns) || RESERVED_PREFIXES.iter().any(|p| ns.starts_with(p)) {
        return Err(SpecError::ReservedNamespace(ns.to_owned()));
    }
    if spec.tables.is_empty() {
        return Err(SpecError::NoTables);
    }

    let mut warnings = Vec::new();
    let mut table_names: Vec<&str> = Vec::new();
    for table in &spec.tables {
        if table_names.contains(&table.name.as_str()) {
            return Err(SpecError::Duplicate {
                kind: "table",
                value: table.name.clone(),
            });
        }
        table_names.push(&table.name);
        validate_table(table, &mut warnings)?;
    }
    Ok(warnings)
}

fn validate_table(table: &TableSpec, warnings: &mut Vec<String>) -> Result<(), SpecError> {
    let name = table.name.as_str();
    if name.contains('.') || !is_safe_identifier(name) {
        return Err(SpecError::BadIdentifier {
            kind: "table",
            value: name.to_owned(),
        });
    }
    if table.columns.is_empty() {
        return Err(SpecError::NoColumns(name.to_owned()));
    }
    if !table.tenant_scoped {
        if !table.acknowledge_unscoped {
            return Err(SpecError::UnscopedNotAcknowledged(name.to_owned()));
        }
        warnings.push(format!(
            "table `{name}` is not tenant-scoped (acknowledged); without RLS it will \
             not be exposed by the reflection surfaces"
        ));
    }

    let mut column_names: Vec<&str> = Vec::new();
    for col in &table.columns {
        let cname = col.name.as_str();
        if cname.contains('.') || !is_safe_identifier(cname) {
            return Err(SpecError::BadIdentifier {
                kind: "column",
                value: cname.to_owned(),
            });
        }
        if table.tenant_scoped && cname == "tenant_id" {
            return Err(SpecError::TenantIdReserved(name.to_owned()));
        }
        if column_names.contains(&cname) {
            return Err(SpecError::Duplicate {
                kind: "column",
                value: cname.to_owned(),
            });
        }
        column_names.push(cname);
        if let Some(default) = &col.default {
            if !default_is_allowed(default) {
                return Err(SpecError::BadDefault {
                    column: format!("{name}.{cname}"),
                    value: default.clone(),
                });
            }
        }
    }

    let mut index_names: Vec<&str> = Vec::new();
    for idx in &table.indexes {
        let iname = idx.name.as_str();
        if iname.contains('.') || !is_safe_identifier(iname) {
            return Err(SpecError::BadIdentifier {
                kind: "index",
                value: iname.to_owned(),
            });
        }
        if index_names.contains(&iname) {
            return Err(SpecError::Duplicate {
                kind: "index",
                value: iname.to_owned(),
            });
        }
        if table.tenant_scoped && iname == format!("{name}_tenant_idx") {
            return Err(SpecError::ReservedIndexName(iname.to_owned()));
        }
        index_names.push(iname);
        for icol in &idx.columns {
            let icol = icol.as_str();
            if icol.contains('.') || !is_safe_identifier(icol) {
                return Err(SpecError::BadIdentifier {
                    kind: "index column",
                    value: icol.to_owned(),
                });
            }
            let is_generated_tenant_col = table.tenant_scoped && icol == "tenant_id";
            if !column_names.contains(&icol) && !is_generated_tenant_col {
                return Err(SpecError::UnknownIndexColumn {
                    index: iname.to_owned(),
                    column: icol.to_owned(),
                });
            }
        }
    }
    Ok(())
}

/// A default expression is allowed when it is an allowlisted function call, a
/// boolean, an integer/decimal literal, or a single-quoted string literal
/// with no embedded quote characters (so `''`-escaping games, `--` comment
/// terminators inside quotes, and `$$` quoting are all structurally
/// impossible).
fn default_is_allowed(expr: &str) -> bool {
    if ALLOWED_DEFAULT_FNS.contains(&expr) {
        return true;
    }
    if expr == "true" || expr == "false" {
        return true;
    }
    if is_numeric_literal(expr) {
        return true;
    }
    is_simple_string_literal(expr)
}

fn is_numeric_literal(expr: &str) -> bool {
    let body = expr.strip_prefix('-').unwrap_or(expr);
    if body.is_empty() {
        return false;
    }
    let mut dots = 0usize;
    for c in body.chars() {
        match c {
            '0'..='9' => {}
            '.' => dots += 1,
            _ => return false,
        }
    }
    dots <= 1 && body != "."
}

fn is_simple_string_literal(expr: &str) -> bool {
    let Some(inner) = expr.strip_prefix('\'').and_then(|rest| rest.strip_suffix('\'')) else {
        return false;
    };
    // No quotes of any kind inside, no backslashes, no control characters:
    // the literal is embedded verbatim, so its body must be inert.
    !inner
        .chars()
        .any(|c| c == '\'' || c == '\\' || c.is_control())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provision::spec::{ColumnSpec, ColumnType, IndexSpec, Namespace};

    fn minimal_table(name: &str) -> TableSpec {
        TableSpec {
            name: name.into(),
            comment: None,
            tenant_scoped: true,
            acknowledge_unscoped: false,
            columns: vec![ColumnSpec {
                name: "id".into(),
                column_type: ColumnType::Text,
                nullable: false,
                primary_key: true,
                default: None,
            }],
            indexes: vec![],
            api_exposed: true,
        }
    }

    fn spec_with(ns: &str, tables: Vec<TableSpec>) -> SchemaSpec {
        SchemaSpec {
            namespace: Namespace(ns.into()),
            tables,
        }
    }

    #[test]
    fn accepts_a_minimal_tenant_scoped_spec() {
        let spec = spec_with("sansaba_sourcing", vec![minimal_table("permit_watch")]);
        assert_eq!(validate_spec(&spec), Ok(vec![]));
    }

    #[test]
    fn refuses_reserved_namespaces_before_anything_else() {
        for ns in ["public", "information_schema", "flint_meta", "pg_catalog", "flint_"] {
            let spec = spec_with(ns, vec![minimal_table("t")]);
            assert_eq!(
                validate_spec(&spec),
                Err(SpecError::ReservedNamespace(ns.into())),
                "{ns} must be reserved"
            );
        }
    }

    #[test]
    fn refuses_injection_shaped_identifiers() {
        for bad in [
            "permit; DROP TABLE users",
            "permit\"watch",
            "permit'watch",
            "permit--watch",
            "permit$$watch",
            "permit watch",
            "pérmit", // unicode homoglyph territory: non-ASCII rejected outright
            "",
        ] {
            let spec = spec_with("ok_ns", vec![minimal_table(bad)]);
            assert!(
                validate_spec(&spec).is_err(),
                "table name {bad:?} must be rejected"
            );
        }
    }

    #[test]
    fn refuses_dotted_namespace_smuggling() {
        // is_safe_identifier accepts `a.b`; the namespace rule must not.
        let spec = spec_with("a.b", vec![minimal_table("t")]);
        assert_eq!(validate_spec(&spec), Err(SpecError::BadNamespace("a.b".into())));
    }

    #[test]
    fn tenant_id_cannot_be_caller_declared_when_scoped() {
        let mut t = minimal_table("t");
        t.columns.push(ColumnSpec {
            name: "tenant_id".into(),
            column_type: ColumnType::Text,
            nullable: false,
            primary_key: false,
            default: None,
        });
        let spec = spec_with("ok_ns", vec![t]);
        assert_eq!(validate_spec(&spec), Err(SpecError::TenantIdReserved("t".into())));
    }

    #[test]
    fn unscoped_requires_acknowledgement_and_warns() {
        let mut t = minimal_table("t");
        t.tenant_scoped = false;
        let spec = spec_with("ok_ns", vec![t.clone()]);
        assert_eq!(
            validate_spec(&spec),
            Err(SpecError::UnscopedNotAcknowledged("t".into()))
        );

        t.acknowledge_unscoped = true;
        let spec = spec_with("ok_ns", vec![t]);
        let warnings = validate_spec(&spec).expect("acknowledged unscoped is allowed");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("not tenant-scoped"));
    }

    #[test]
    fn default_allowlist_holds() {
        for ok in ["now()", "gen_random_uuid()", "'{}'", "'abc'", "0", "-3.5", "true"] {
            assert!(default_is_allowed(ok), "{ok} should be allowed");
        }
        for bad in [
            "now(); DROP TABLE x",
            "'a''b'",
            "'a\\'b'",
            "''; DROP TABLE x; --'",
            "$$x$$",
            "random()",
            "1;2",
            "'--'||version()",
            "'\u{0007}'",
        ] {
            assert!(!default_is_allowed(bad), "{bad} must be rejected");
        }
    }

    #[test]
    fn index_columns_must_exist_but_tenant_id_is_implicit_when_scoped() {
        let mut t = minimal_table("t");
        t.indexes.push(IndexSpec {
            name: "t_tenant_idx2".into(),
            columns: vec!["tenant_id".into()],
            unique: false,
        });
        let spec = spec_with("ok_ns", vec![t.clone()]);
        assert!(validate_spec(&spec).is_ok(), "tenant_id is implicit on scoped tables");

        t.indexes.push(IndexSpec {
            name: "t_bad_idx".into(),
            columns: vec!["missing".into()],
            unique: false,
        });
        let spec = spec_with("ok_ns", vec![t]);
        assert_eq!(
            validate_spec(&spec),
            Err(SpecError::UnknownIndexColumn {
                index: "t_bad_idx".into(),
                column: "missing".into()
            })
        );
    }
}
