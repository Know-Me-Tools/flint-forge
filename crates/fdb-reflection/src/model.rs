use serde::{Deserialize, Serialize};

/// A live snapshot of the full database structure, assembled by `ReflectionEngine::reflect()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseModel {
    /// Every user table visible to `flint_meta.tables()`, each with its columns,
    /// keys, and RLS status already populated by `ReflectionEngine::reflect()`.
    pub tables: Vec<Table>,
    /// Every SQL function visible to `flint_meta.functions()`, exposed by the
    /// REST compiler as `/rpc/<schema>/<name>` endpoints.
    pub functions: Vec<FnMeta>,
    /// Every view visible to `flint_meta.views()`.
    pub views: Vec<ViewMeta>,
    /// Monotonically increasing schema version from `flint_meta.version()`.
    /// Bumped by `ext-flint-meta` on any DDL change; drives `StateManager`'s
    /// ArcSwap hot-reload (a `reflect()` producing the same version is a no-op).
    pub version: u64,
}

/// A single reflected table (or, for RLS/key purposes, any relation `flint_meta`
/// treats as table-shaped).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    /// Postgres schema the table lives in (e.g. `public`).
    pub schema: String,
    /// Table name, unqualified.
    pub name: String,
    /// Every column, populated by a follow-up `flint_meta.columns($1, $2)` call
    /// per table (see `ReflectionEngine::fetch_columns`).
    pub columns: Vec<Column>,
    /// Names of the columns making up the primary key, in key order.
    pub pk: Vec<String>,
    /// Foreign keys declared on this table.
    pub fk: Vec<ForeignKey>,
    /// Whether row-level security is enabled on this table. REST/GraphQL
    /// compilation and the subscription RLS re-query both depend on this flag.
    pub rls_enabled: bool,
    /// Ciphertext-only DEK. Never contains plaintext key material.
    pub vault_key: Option<EncryptedDek>,
}

/// A single reflected column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    /// Column name, unqualified.
    pub name: String,
    /// Raw Postgres type name as reported by `flint_meta.columns()`, e.g.
    /// `text`, `integer`, or `vector(1536)` for a pgvector embedding column
    /// (see [`is_vector_type`]).
    pub pg_type: String,
    /// Whether the column allows `NULL`.
    pub nullable: bool,
    /// The column's `DEFAULT` expression as SQL text, if any.
    pub default: Option<String>,
}

/// A single reflected foreign-key constraint, sourced from `flint_meta.tables()`
/// (or a related catalog call) for one owning [`Table`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKey {
    /// The referencing column on the owning table.
    pub from_col: String,
    /// Schema of the referenced table.
    pub to_schema: String,
    /// Name of the referenced table.
    pub to_table: String,
    /// Column on the referenced table that `from_col` points to.
    pub to_col: String,
}

/// Returns `true` when `pg_type` is a pgvector dimension-typed embedding column.
/// Matches both plain `"vector"` and `"vector(N)"` forms.
pub fn is_vector_type(pg_type: &str) -> bool {
    pg_type == "vector" || pg_type.starts_with("vector(")
}

/// XChaCha20-Poly1305 ciphertext blob only.
///
/// SECURITY: Plaintext DEK MUST NOT appear here or anywhere in `DatabaseModel`
/// or `CompiledState`. The plaintext is only available transiently during
/// flint_vault KMS-unwrap operations and is never stored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedDek(pub Vec<u8>);

/// A single reflected SQL function, exposed as an `/rpc/<schema>/<name>` REST
/// endpoint by the endpoint-generation pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FnMeta {
    /// Postgres schema the function lives in.
    pub schema: String,
    /// Function name, unqualified. Overloaded functions with the same name are
    /// not currently disambiguated by argument signature at this layer.
    pub name: String,
    /// Function arguments in declaration order, populated by a follow-up
    /// `flint_meta.function_args($1, $2)` call per function.
    pub args: Vec<ArgMeta>,
    /// Raw Postgres return type name (may be a scalar, composite, or `SETOF`/
    /// table type as reported by `flint_meta.functions()`).
    pub return_type: String,
    /// Whether the function is `SECURITY DEFINER` — it runs with the
    /// privileges of its owner rather than the caller, which the permission-
    /// analysis pass treats as a privilege-escalation risk to flag/gate.
    pub security_definer: bool,
}

/// A single reflected function argument.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgMeta {
    /// Argument name as declared in the function signature.
    pub name: String,
    /// Raw Postgres type name, e.g. `text` or `vector(1536)` for a pgvector
    /// query-embedding argument (see [`is_vector_type`]).
    pub pg_type: String,
}

/// A single reflected view (plain or materialized, as reported by
/// `flint_meta.views()`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewMeta {
    /// Postgres schema the view lives in.
    pub schema: String,
    /// View name, unqualified.
    pub name: String,
    /// The view's output columns. Unlike [`Table::columns`], this is not
    /// currently populated by a follow-up fetch in `ReflectionEngine::reflect`.
    pub columns: Vec<Column>,
    /// Whether the view was created `WITH (security_barrier = true)`, which
    /// prevents leaky-function optimization from bypassing its `WHERE`
    /// clause — relevant to whether the view safely enforces row filtering.
    pub security_barrier: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_vector_type_matches_plain_and_dimensioned() {
        assert!(is_vector_type("vector"));
        assert!(is_vector_type("vector(3)"));
        assert!(is_vector_type("vector(1536)"));
        assert!(!is_vector_type("text"));
        assert!(!is_vector_type("integer"));
        assert!(!is_vector_type("vectorize_result")); // must not match prefix-only
    }

    #[test]
    fn arg_meta_stores_vector_pg_type() {
        let arg = ArgMeta {
            name: "query_vec".into(),
            pg_type: "vector(3)".into(),
        };
        assert!(is_vector_type(&arg.pg_type));
        assert_eq!(arg.pg_type, "vector(3)");
    }
}
