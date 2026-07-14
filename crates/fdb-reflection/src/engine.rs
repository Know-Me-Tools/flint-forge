use sqlx::PgPool;
use tracing::instrument;

use crate::{
    compiled::{A2uiCatalog, A2uiCatalogEntry},
    error::ReflectionError,
    model::{ArgMeta, Column, DatabaseModel, FnMeta, Table, ViewMeta},
    passes,
};

/// Queries the `flint_meta` schema (installed by Phase 1 `ext-flint-meta`)
/// and assembles a `DatabaseModel` IR.
///
/// Must be called with service_role credentials so it can read `flint_meta`
/// without RLS blocking the reflection queries.
pub struct ReflectionEngine {
    pool: PgPool,
}

impl ReflectionEngine {
    /// Wrap a `PgPool` in a `ReflectionEngine`. The pool MUST be authenticated
    /// as `service_role` (or an equivalent privileged role) so `flint_meta.*`
    /// catalog queries are not blocked by RLS.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Reflect the live database into a `DatabaseModel`.
    /// Applies normalization, validation, and permission-analysis passes.
    ///
    /// # Errors
    ///
    /// Returns [`ReflectionError::Query`] if any `flint_meta.*` catalog query
    /// fails, or [`ReflectionError::Validation`] if the assembled model fails
    /// [`crate::passes::validation`] (e.g. an empty table or a dangling
    /// foreign key).
    #[instrument(skip(self), err)]
    pub async fn reflect(&self) -> Result<DatabaseModel, ReflectionError> {
        let version = self.fetch_version().await?;
        let mut tables = self.fetch_tables().await?;
        let functions = self.fetch_functions().await?;
        let views = self.fetch_views().await?;

        for table in &mut tables {
            table.columns = self.fetch_columns(&table.schema, &table.name).await?;
        }

        let mut model = DatabaseModel {
            tables,
            functions,
            views,
            version,
        };

        passes::normalization::run(&mut model);
        passes::validation::run(&model)?;
        passes::permission_analysis::run(&model);

        Ok(model)
    }

    async fn fetch_version(&self) -> Result<u64, ReflectionError> {
        let row: (i64,) = sqlx::query_as("SELECT version FROM flint_meta.version()")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0 as u64)
    }

    async fn fetch_tables(&self) -> Result<Vec<Table>, ReflectionError> {
        let rows: Vec<(String, String, bool)> =
            sqlx::query_as("SELECT schema_name, table_name, rls_enabled FROM flint_meta.tables()")
                .fetch_all(&self.pool)
                .await?;

        Ok(rows
            .into_iter()
            .map(|(schema, name, rls_enabled)| Table {
                schema,
                name,
                columns: vec![],
                pk: vec![],
                fk: vec![],
                rls_enabled,
                vault_key: None,
            })
            .collect())
    }

    async fn fetch_columns(
        &self,
        schema: &str,
        table: &str,
    ) -> Result<Vec<Column>, ReflectionError> {
        let rows: Vec<(String, String, bool, Option<String>)> = sqlx::query_as(
            "SELECT column_name, pg_type, is_nullable, column_default \
             FROM flint_meta.columns($1, $2)",
        )
        .bind(schema)
        .bind(table)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(name, pg_type, nullable, default)| Column {
                name,
                pg_type,
                nullable,
                default,
            })
            .collect())
    }

    async fn fetch_functions(&self) -> Result<Vec<FnMeta>, ReflectionError> {
        let rows: Vec<(String, String, String, bool)> = sqlx::query_as(
            "SELECT schema_name, function_name, return_type, security_definer \
             FROM flint_meta.functions()",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut functions: Vec<FnMeta> = rows
            .into_iter()
            .map(|(schema, name, return_type, security_definer)| FnMeta {
                schema,
                name,
                args: vec![],
                return_type,
                security_definer,
            })
            .collect();

        // Fetch args for each function, capturing vector types like "vector(N)"
        for fn_meta in &mut functions {
            fn_meta.args = self
                .fetch_function_args(&fn_meta.schema, &fn_meta.name)
                .await?;
        }

        Ok(functions)
    }

    async fn fetch_function_args(
        &self,
        schema: &str,
        fn_name: &str,
    ) -> Result<Vec<ArgMeta>, ReflectionError> {
        // flint_meta.function_args() returns arg_name and arg_type (pg_type string).
        // Vector args come through as "vector(N)" e.g. "vector(1536)".
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT arg_name, arg_type \
             FROM flint_meta.function_args($1, $2)",
        )
        .bind(schema)
        .bind(fn_name)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(name, pg_type)| ArgMeta { name, pg_type })
            .collect())
    }

    async fn fetch_views(&self) -> Result<Vec<ViewMeta>, ReflectionError> {
        let rows: Vec<(String, String, bool)> = sqlx::query_as(
            "SELECT schema_name, view_name, security_barrier FROM flint_meta.views()",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(schema, name, security_barrier)| ViewMeta {
                schema,
                name,
                columns: vec![],
                security_barrier,
            })
            .collect())
    }

    /// Load the A2UI component catalog from `flint_a2ui.components`.
    ///
    /// Returns an empty catalog with graceful degradation when the schema is not
    /// yet deployed (the flint_a2ui migration has not run). Uses dynamic `query_as`
    /// rather than the `query_as!` macro because the schema is created at runtime.
    ///
    /// # Errors
    ///
    /// Returns [`ReflectionError::Query`] if the `flint_a2ui.components` select
    /// fails after the schema is confirmed to exist. The prior
    /// `information_schema.tables` existence check never errors this call —
    /// a connectivity failure there degrades to "schema not found" (empty
    /// catalog) via `unwrap_or(false)`.
    pub async fn load_a2ui_catalog(&self) -> Result<A2uiCatalog, ReflectionError> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.tables
                WHERE table_schema = 'flint_a2ui' AND table_name = 'components'
            )",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(false);

        if !exists {
            tracing::warn!(
                "flint_a2ui schema not found; A2UI catalog will be empty until migration 0002 runs"
            );
            return Ok(A2uiCatalog::empty());
        }

        // Dynamic query_as (not query_as!) — schema exists at runtime not compile time.
        let rows: Vec<(String, String, String, serde_json::Value, Option<String>)> =
            sqlx::query_as(
                "SELECT slug, primitive_type, category, schema, description
                 FROM flint_a2ui.components
                 WHERE is_base = true OR application_id IS NULL
                 ORDER BY category, slug",
            )
            .fetch_all(&self.pool)
            .await?;

        let components = rows
            .into_iter()
            .map(
                |(slug, primitive_type, category, schema, description)| A2uiCatalogEntry {
                    slug,
                    primitive_type,
                    category,
                    schema,
                    description,
                },
            )
            .collect();

        Ok(A2uiCatalog {
            catalog_id: "/a2ui/v1/catalog/flint-base/1.0".into(),
            version: "1.0.0".into(),
            components,
        })
    }
}
