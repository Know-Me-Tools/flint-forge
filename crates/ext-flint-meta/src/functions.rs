//! Reflection query functions for ext-flint-meta.
//!
//! The set-returning query functions (`tables()`, `columns()`, `relationships()`,
//! `functions()`) are declared as PL/pgSQL SQL functions via `extension_sql!` so
//! callers receive proper `SETOF` composite types without requiring the complex
//! pgrx composite-return machinery.
//!
//! The two simple point-query functions — `check_permission()` and
//! `set_identity()` — are Rust `#[pg_extern]` because their return types are
//! trivial (`bool`) and SPI parameterisation is straightforward.
//!
//! ## Security invariants
//!
//! - `check_permission` and `set_identity` use typed SPI bind parameters;
//!   user input is **never** interpolated into a SQL string.
//! - The `claims_json` argument of `set_identity` is intentionally never
//!   logged — it may contain JWT claim values.
//! - SQL wrappers are `SECURITY INVOKER` and `STABLE PARALLEL SAFE` where
//!   correct; `set_identity` is Rust-only because `set_config` is not stable.

use pgrx::datum::DatumWithOid;
use pgrx::prelude::*;

/// Check whether a Keto permission tuple exists in the local cache.
///
/// Returns `true` when `(namespace, object_id, relation, subject_id)` is found
/// in `flint_meta.keto_tuples`. Used by the reflection engine to gate access
/// without a round-trip to the Keto service.
#[pg_extern]
fn check_permission(
    namespace: &str,
    object_id: &str,
    relation: &str,
    subject_id: &str,
) -> bool {
    let sql = "SELECT COUNT(*) > 0 \
               FROM flint_meta.keto_tuples \
               WHERE namespace  = $1 \
                 AND object_id  = $2 \
                 AND relation   = $3 \
                 AND subject_id = $4";

    let args: &[DatumWithOid<'_>] = &[
        DatumWithOid::from(namespace),
        DatumWithOid::from(object_id),
        DatumWithOid::from(relation),
        DatumWithOid::from(subject_id),
    ];

    Spi::get_one_with_args::<bool>(sql, args)
        .unwrap_or_else(|_| None)
        .unwrap_or(false)
}

/// Set the JWT claims GUC for the current transaction.
///
/// Equivalent to `SET LOCAL "request.jwt.claims" = claims_json`. Used by the
/// flint-reflection engine to impersonate a caller for RLS checks during
/// subscription re-validation.
///
/// Returns `true` on success, `false` on SPI error.
///
/// # Security
///
/// The `claims_json` value is bound as a typed SPI parameter and is **never**
/// logged — it may contain JWT claim values. Callers must ensure the JSON
/// content is validated before passing it to this function.
#[pg_extern]
fn set_identity(claims_json: &str) -> bool {
    let sql = "SELECT set_config('request.jwt.claims', $1, true)";
    let args: &[DatumWithOid<'_>] = &[DatumWithOid::from(claims_json)];
    Spi::run_with_args(sql, args).is_ok()
}

extension_sql!(
    r#"
-- ── tables(schema_filter text DEFAULT NULL) ──────────────────────────────────
-- Returns rows from the pre-computed cache_tables. STABLE PARALLEL SAFE because
-- it only reads flint_meta.cache_tables and takes no locks beyond a snapshot.
CREATE OR REPLACE FUNCTION flint_meta.tables(schema_filter text DEFAULT NULL)
RETURNS TABLE (
    schema_name  text,
    table_name   text,
    is_view      bool,
    description  text,
    rls_enabled  bool
)
LANGUAGE sql
STABLE PARALLEL SAFE
SECURITY INVOKER
AS $$
    SELECT schema_name, table_name, is_view, description, rls_enabled
    FROM   flint_meta.cache_tables
    WHERE  schema_filter IS NULL OR schema_name = schema_filter
    ORDER  BY schema_name, table_name;
$$;

GRANT EXECUTE ON FUNCTION flint_meta.tables(text) TO authenticated, anon, service_role;

-- ── columns(p_schema text, p_table text) ─────────────────────────────────────
-- Shape matches what the fdb-reflection engine consumes:
-- (column_name, pg_type, is_nullable, column_default).
CREATE OR REPLACE FUNCTION flint_meta.columns(p_schema text, p_table text)
RETURNS TABLE (
    column_name    text,
    pg_type        text,
    is_nullable    bool,
    column_default text
)
LANGUAGE sql
STABLE PARALLEL SAFE
SECURITY INVOKER
AS $$
    SELECT column_name, data_type AS pg_type, is_nullable, column_default
    FROM   flint_meta.cache_columns
    WHERE  schema_name = p_schema AND table_name = p_table
    ORDER  BY ordinal;
$$;

GRANT EXECUTE ON FUNCTION flint_meta.columns(text, text) TO authenticated, anon, service_role;

-- ── relationships(p_schema text, p_table text) ───────────────────────────────
CREATE OR REPLACE FUNCTION flint_meta.relationships(p_schema text, p_table text)
RETURNS TABLE (
    from_schema      text,
    from_table       text,
    from_column      text,
    to_schema        text,
    to_table         text,
    to_column        text,
    constraint_name  text
)
LANGUAGE sql
STABLE PARALLEL SAFE
SECURITY INVOKER
AS $$
    SELECT from_schema, from_table, from_column,
           to_schema, to_table, to_column, constraint_name
    FROM   flint_meta.cache_relationships
    WHERE  from_schema = p_schema AND from_table = p_table;
$$;

GRANT EXECUTE ON FUNCTION flint_meta.relationships(text, text) TO authenticated, anon, service_role;

-- ── functions(p_schema text DEFAULT NULL) ────────────────────────────────────
-- Shape matches what the fdb-reflection engine consumes:
-- (schema_name, function_name, return_type, security_definer).
CREATE OR REPLACE FUNCTION flint_meta.functions(p_schema text DEFAULT NULL)
RETURNS TABLE (
    schema_name      text,
    function_name    text,
    return_type      text,
    security_definer bool
)
LANGUAGE sql
STABLE PARALLEL SAFE
SECURITY INVOKER
AS $$
    SELECT schema_name,
           function_name,
           return_type,
           false AS security_definer
    FROM   flint_meta.cache_functions
    WHERE  p_schema IS NULL OR schema_name = p_schema
    ORDER  BY schema_name, function_name;
$$;

GRANT EXECUTE ON FUNCTION flint_meta.functions(text) TO authenticated, anon, service_role;

-- ── function_args(p_schema text, p_function text) ────────────────────────────
-- Returns argument metadata for a single function by querying pg_proc directly.
-- Vector args come through as "vector(N)" e.g. "vector(1536)".
CREATE OR REPLACE FUNCTION flint_meta.function_args(p_schema text, p_function text)
RETURNS TABLE (
    arg_name text,
    arg_type text
)
LANGUAGE sql
STABLE PARALLEL SAFE
SECURITY INVOKER
SET search_path = flint_meta, pg_catalog
AS $$
    SELECT COALESCE(p.proargnames[a.ord], 'arg' || a.ord) AS arg_name,
           pg_catalog.format_type(a.atttypid, NULL) AS arg_type
    FROM   pg_proc     p
    JOIN   pg_namespace n ON n.oid = p.pronamespace
    CROSS JOIN LATERAL unnest(p.proargtypes)
                        WITH ORDINALITY AS a(atttypid, ord)
    WHERE  n.nspname = p_schema
      AND  p.proname = p_function
    ORDER  BY a.ord;
$$;

GRANT EXECUTE ON FUNCTION flint_meta.function_args(text, text) TO authenticated, anon, service_role;

-- ── views() ──────────────────────────────────────────────────────────────────
-- Shape matches what the fdb-reflection engine consumes:
-- (schema_name, view_name, security_barrier).
CREATE OR REPLACE FUNCTION flint_meta.views()
RETURNS TABLE (
    schema_name      text,
    view_name        text,
    security_barrier bool
)
LANGUAGE sql
STABLE PARALLEL SAFE
SECURITY INVOKER
AS $$
    SELECT schema_name,
           table_name AS view_name,
           false AS security_barrier
    FROM   flint_meta.cache_tables
    WHERE  is_view = true
    ORDER  BY schema_name, table_name;
$$;

GRANT EXECUTE ON FUNCTION flint_meta.views() TO authenticated, anon, service_role;
"#,
    name = "flint_meta_functions",
    requires = ["flint_meta_triggers"]
);

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_check_permission_true() {
        Spi::run(
            "INSERT INTO flint_meta.keto_tuples \
             (namespace, object_id, relation, subject_id) \
             VALUES ('documents', 'doc-1', 'viewer', 'user-abc') \
             ON CONFLICT DO NOTHING",
        )
        .unwrap();

        let result = crate::functions::check_permission(
            "documents",
            "doc-1",
            "viewer",
            "user-abc",
        );
        assert!(result, "check_permission should return true for an existing tuple");

        Spi::run(
            "DELETE FROM flint_meta.keto_tuples \
             WHERE namespace = 'documents' AND object_id = 'doc-1'",
        )
        .unwrap();
    }

    #[pg_test]
    fn test_check_permission_false() {
        let result = crate::functions::check_permission(
            "documents",
            "nonexistent",
            "viewer",
            "nobody",
        );
        assert!(!result, "check_permission should return false for a missing tuple");
    }

    #[pg_test]
    fn test_set_identity_sets_guc() {
        let claims = r#"{"sub":"user-xyz","role":"authenticated","tenant_id":"org-1"}"#;
        let ok = crate::functions::set_identity(claims);
        assert!(ok, "set_identity should return true on success");

        let stored = Spi::get_one::<String>(
            "SELECT current_setting('request.jwt.claims', true)",
        )
        .unwrap_or(None)
        .unwrap_or_default();

        assert_eq!(stored, claims);
    }

    #[pg_test]
    fn test_version_at_least_one() {
        let v = crate::version::version();
        assert!(v >= 1, "version() should return >= 1 after bootstrap");
    }
}
