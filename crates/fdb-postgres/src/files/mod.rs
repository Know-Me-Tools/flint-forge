//! RLS-scoped, transactional file metadata adapter.
mod mutations;
use crate::{conn::PgConn, PgBackend};
use async_trait::async_trait;
use deadpool_postgres::Pool;
use fdb_domain::files::{FileId, FileQuery, FileRecord, FileType, PutFile};
use fdb_ports::{
    files::{FileError, FileMetadataStore},
    DatabaseBackend,
};
use forge_identity::RlsContext;
use serde_json::Value;

/// PostgreSQL persistence for private typed file instances.
pub struct PgFileMetadata {
    backend: PgBackend,
}
impl PgFileMetadata {
    /// Use an existing configured pool; no connections or DDL occur here.
    #[must_use]
    pub fn new(pool: Pool) -> Self {
        Self {
            backend: PgBackend { pool },
        }
    }
}

fn owner(rls: &RlsContext) -> Result<(String, String), FileError> {
    if rls.role != "authenticated" {
        return Err(FileError::Denied);
    }
    let tenant = rls.tenant().ok_or(FileError::Denied)?.0;
    let subject = rls.subject().ok_or(FileError::Denied)?.0;
    if tenant.is_empty() || subject.is_empty() {
        return Err(FileError::Denied);
    }
    Ok((tenant, subject))
}
fn pg_error(error: tokio_postgres::Error) -> FileError {
    match error.code().map(tokio_postgres::error::SqlState::code) {
        Some("23505" | "40001" | "40P01") => FileError::Conflict,
        // PostgreSQL JSONB rejects U+0000 as untranslatable_character (22P05).
        // Map only that specific additional code, not the entire data-error class.
        Some("23514" | "23503" | "22P02" | "22P05" | "22003" | "22023") => FileError::Invalid,
        Some("42501") => FileError::Denied,
        _ => FileError::Unavailable,
    }
}
fn decode<T: serde::de::DeserializeOwned>(row: &tokio_postgres::Row) -> Result<T, FileError> {
    let value: Value = row.try_get(0).map_err(|_| FileError::Unavailable)?;
    serde_json::from_value(value).map_err(|_| FileError::Unavailable)
}

#[async_trait]
impl FileMetadataStore for PgFileMetadata {
    #[tracing::instrument(skip_all)]
    async fn put_type(&self, rls: &RlsContext, value: &FileType) -> Result<FileType, FileError> {
        self.write_type(rls, value).await
    }
    #[tracing::instrument(skip_all)]
    async fn types(&self, rls: &RlsContext) -> Result<Vec<FileType>, FileError> {
        let (tenant, subject) = owner(rls)?;
        let conn = self
            .backend
            .acquire(rls)
            .await
            .map_err(|_| FileError::Unavailable)?;
        let pg = PgConn::from_conn(&conn).ok_or(FileError::Unavailable)?;
        let rows = pg.client().map_err(|_| FileError::Unavailable)?.query(
            "SELECT definition FROM flint_files.types WHERE tenant_id=$1 AND subject=$2 ORDER BY type_id,version LIMIT 1001", &[&tenant,&subject]).await.map_err(pg_error)?;
        // Never silently truncate matching: completeness is required to detect ambiguity.
        if rows.len() > 1000 {
            return Err(FileError::Invalid);
        }
        let result = rows.iter().map(decode).collect::<Result<_, _>>()?;
        pg.commit().await.map_err(|_| FileError::Unavailable)?;
        Ok(result)
    }
    #[tracing::instrument(skip_all)]
    async fn put(
        &self,
        rls: &RlsContext,
        id: &FileId,
        value: &PutFile,
    ) -> Result<FileRecord, FileError> {
        self.write_record(rls, id, value).await
    }
    #[tracing::instrument(skip_all)]
    async fn get(&self, rls: &RlsContext, id: &FileId) -> Result<FileRecord, FileError> {
        let (tenant, subject) = owner(rls)?;
        let conn = self
            .backend
            .acquire(rls)
            .await
            .map_err(|_| FileError::Unavailable)?;
        let pg = PgConn::from_conn(&conn).ok_or(FileError::Unavailable)?;
        let row = pg.client().map_err(|_| FileError::Unavailable)?.query_opt(
            "SELECT data || jsonb_build_object('file_id',file_id,'revision',revision) FROM flint_files.records WHERE tenant_id=$1 AND subject=$2 AND file_id=$3", &[&tenant,&subject,&id.0]).await.map_err(pg_error)?.ok_or(FileError::NotFound)?;
        let result = decode(&row)?;
        pg.commit().await.map_err(|_| FileError::Unavailable)?;
        Ok(result)
    }
    #[tracing::instrument(skip_all)]
    async fn query(
        &self,
        rls: &RlsContext,
        query: &FileQuery,
    ) -> Result<Vec<FileRecord>, FileError> {
        let (tenant, subject) = owner(rls)?;
        if !(1..=100).contains(&query.limit) || !query.attributes.is_object() {
            return Err(FileError::Invalid);
        }
        let conn = self
            .backend
            .acquire(rls)
            .await
            .map_err(|_| FileError::Unavailable)?;
        let pg = PgConn::from_conn(&conn).ok_or(FileError::Unavailable)?;
        let typ = query.type_id.as_ref().map(|v| v.0.as_str());
        let after = query.after.as_ref().map(|v| v.0.as_str());
        let tags = serde_json::to_value(&query.tags).map_err(|_| FileError::Invalid)?;
        let categories = serde_json::to_value(&query.categories).map_err(|_| FileError::Invalid)?;
        let rows = pg.client().map_err(|_| FileError::Unavailable)?.query(
            "SELECT data || jsonb_build_object('file_id',file_id,'revision',revision) FROM flint_files.records WHERE tenant_id=$1 AND subject=$2 AND ($3::text IS NULL OR data->>'cid'=$3) AND ($4::text IS NULL OR type_id=$4) AND data->'tags' @> $5 AND data->'categories' @> $6 AND data->'attributes' @> $7 AND ($8::text IS NULL OR file_id>$8) ORDER BY file_id LIMIT $9",
            &[&tenant,&subject,&query.cid,&typ,&tags,&categories,&query.attributes,&after,&query.limit]).await.map_err(pg_error)?;
        let result = rows.iter().map(decode).collect::<Result<_, _>>()?;
        pg.commit().await.map_err(|_| FileError::Unavailable)?;
        Ok(result)
    }
}
