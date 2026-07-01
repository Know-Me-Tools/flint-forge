use serde::{Deserialize, Serialize};

/// A live snapshot of the full database structure, assembled by `ReflectionEngine::reflect()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseModel {
    pub tables: Vec<Table>,
    pub functions: Vec<FnMeta>,
    pub views: Vec<ViewMeta>,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub schema: String,
    pub name: String,
    pub columns: Vec<Column>,
    pub pk: Vec<String>,
    pub fk: Vec<ForeignKey>,
    pub rls_enabled: bool,
    /// Ciphertext-only DEK. Never contains plaintext key material.
    pub vault_key: Option<EncryptedDek>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub pg_type: String,
    pub nullable: bool,
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKey {
    pub from_col: String,
    pub to_schema: String,
    pub to_table: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FnMeta {
    pub schema: String,
    pub name: String,
    pub args: Vec<ArgMeta>,
    pub return_type: String,
    pub security_definer: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgMeta {
    pub name: String,
    pub pg_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewMeta {
    pub schema: String,
    pub name: String,
    pub columns: Vec<Column>,
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
