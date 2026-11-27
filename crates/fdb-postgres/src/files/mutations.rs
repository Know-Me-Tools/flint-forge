//! Atomic definition publication and record compare-and-swap with retry receipts.
use super::{decode, owner, pg_error, PgFileMetadata};
use crate::conn::PgConn;
use fdb_domain::files::{FileId, FileRecord, FileType, PutFile};
use fdb_ports::{files::FileError, DatabaseBackend};
use forge_identity::RlsContext;
use serde_json::{json, Value};

impl PgFileMetadata {
    pub(super) async fn write_type(
        &self,
        rls: &RlsContext,
        value: &FileType,
    ) -> Result<FileType, FileError> {
        let (tenant, subject) = owner(rls)?;
        let definition = serde_json::to_value(value).map_err(|_| FileError::Invalid)?;
        let conn = self
            .backend
            .acquire(rls)
            .await
            .map_err(|_| FileError::Unavailable)?;
        let pg = PgConn::from_conn(&conn).ok_or(FileError::Unavailable)?;
        let client = pg.client().map_err(|_| FileError::Unavailable)?;
        let key = json!(["type", tenant, subject, value.type_id.0, value.version]).to_string();
        client
            .execute(
                "SELECT pg_advisory_xact_lock(hashtextextended($1,0))",
                &[&key],
            )
            .await
            .map_err(pg_error)?;
        let existing=client.query_opt("SELECT definition FROM flint_files.types WHERE tenant_id=$1 AND subject=$2 AND type_id=$3 AND version=$4", &[&tenant,&subject,&value.type_id.0,&value.version]).await.map_err(pg_error)?;
        if let Some(row) = existing {
            let previous: Value = row.try_get(0).map_err(|_| FileError::Unavailable)?;
            if previous != definition {
                return Err(FileError::Conflict);
            }
        } else {
            client.execute("INSERT INTO flint_files.types(tenant_id,subject,type_id,version,definition) VALUES($1,$2,$3,$4,$5)", &[&tenant,&subject,&value.type_id.0,&value.version,&definition]).await.map_err(pg_error)?;
        }
        pg.commit().await.map_err(|_| FileError::Unavailable)?;
        Ok(value.clone())
    }

    pub(super) async fn write_record(
        &self,
        rls: &RlsContext,
        id: &FileId,
        value: &PutFile,
    ) -> Result<FileRecord, FileError> {
        let (tenant, subject) = owner(rls)?;
        if value.expected_revision < 0 {
            return Err(FileError::Invalid);
        }
        let revision = value
            .expected_revision
            .checked_add(1)
            .ok_or(FileError::Invalid)?;
        let data = serde_json::to_value(value).map_err(|_| FileError::Invalid)?;
        let request = json!({"file_id":id.0,"data":data});
        let conn = self
            .backend
            .acquire(rls)
            .await
            .map_err(|_| FileError::Unavailable)?;
        let pg = PgConn::from_conn(&conn).ok_or(FileError::Unavailable)?;
        let client = pg.client().map_err(|_| FileError::Unavailable)?;
        // All writers acquire receipt then record locks; lock collisions only serialize.
        let retry_key = json!(["receipt", tenant, subject, value.mutation_id]).to_string();
        client
            .execute(
                "SELECT pg_advisory_xact_lock(hashtextextended($1,0))",
                &[&retry_key],
            )
            .await
            .map_err(pg_error)?;
        if let Some(row)=client.query_opt("SELECT request,response FROM flint_files.receipts WHERE tenant_id=$1 AND subject=$2 AND mutation_id=$3", &[&tenant,&subject,&value.mutation_id]).await.map_err(pg_error)? {
            let previous:Value=row.try_get(0).map_err(|_|FileError::Unavailable)?;
            if previous != request { return Err(FileError::Conflict); }
            let response:Value=row.try_get(1).map_err(|_|FileError::Unavailable)?;
            let result=serde_json::from_value(response).map_err(|_|FileError::Unavailable)?;
            pg.commit().await.map_err(|_|FileError::Unavailable)?;
            return Ok(result);
        }
        let file_key = json!(["file", tenant, subject, id.0]).to_string();
        client
            .execute(
                "SELECT pg_advisory_xact_lock(hashtextextended($1,0))",
                &[&file_key],
            )
            .await
            .map_err(pg_error)?;
        let row=if value.expected_revision == 0 {
            client.query_opt("INSERT INTO flint_files.records(tenant_id,subject,file_id,revision,type_id,type_version,data) VALUES($1,$2,$3,1,$4,$5,$6) ON CONFLICT DO NOTHING RETURNING data || jsonb_build_object('file_id',file_id,'revision',revision)", &[&tenant,&subject,&id.0,&value.type_id.0,&value.type_version,&data]).await.map_err(pg_error)?
        } else {
            client.query_opt("UPDATE flint_files.records SET revision=$4,type_id=$5,type_version=$6,data=$7 WHERE tenant_id=$1 AND subject=$2 AND file_id=$3 AND revision=$8 AND data->>'cid'=$9 RETURNING data || jsonb_build_object('file_id',file_id,'revision',revision)", &[&tenant,&subject,&id.0,&revision,&value.type_id.0,&value.type_version,&data,&value.expected_revision,&value.cid]).await.map_err(pg_error)?
        }.ok_or(FileError::Conflict)?;
        let result: FileRecord = decode(&row)?;
        let response = serde_json::to_value(&result).map_err(|_| FileError::Unavailable)?;
        client.execute("INSERT INTO flint_files.receipts(tenant_id,subject,mutation_id,request,response) VALUES($1,$2,$3,$4,$5)", &[&tenant,&subject,&value.mutation_id,&request,&response]).await.map_err(pg_error)?;
        pg.commit().await.map_err(|_| FileError::Unavailable)?;
        Ok(result)
    }
}
