//! Bounded HTTP handlers. Values and identities never enter tracing spans.
use super::{error, FileApiState};
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    Extension, Json,
};
use fdb_domain::files::{FileId, FileQuery, FileType, MatchInput, PutFile};
use fdb_ports::files::FileError;
use forge_identity::RlsContext;
use serde_json::json;

type ApiResult<T> = Result<T, FileError>;
fn response<T: serde::Serialize>(result: ApiResult<T>) -> Response {
    match result {
        Ok(value) => Json(json!({"data":value})).into_response(),
        Err(e) => error(&e),
    }
}

pub(super) async fn capabilities(
    State(state): State<FileApiState>,
    Extension(rls): Extension<RlsContext>,
) -> Response {
    response(state.store(&rls).map(|_|json!({
        "api_version":"files/v1","schema_profile":fdb_app::files::SCHEMA_PROFILE,
        "schema_keywords":["type","properties","required","additionalProperties","items","enum","title","description"],
        "schema_types":["object","array","string","number","integer","boolean","null"],
        "schema_restrictions":{"root_type":"object","type_required_at_each_node":true,"type_arrays":false,"boolean_schemas":false,"additionalProperties":"boolean only","items":"single schema only","max_depth":16,"max_request_bytes":65536},
        "match_criteria":["mime_types","filename_suffixes"],"query_attributes":"jsonb containment",
        "max_query_limit":100,"max_type_versions":1000,"scope":"private tenant and subject","cid_verification":"opaque caller assertion; no content existence check"
    })))
}
#[tracing::instrument(skip_all)]
pub(super) async fn types(
    State(state): State<FileApiState>,
    Extension(rls): Extension<RlsContext>,
) -> Response {
    let result = async { state.store(&rls)?.types(&rls).await }.await;
    response(result)
}
#[tracing::instrument(skip_all)]
pub(super) async fn put_type(
    State(state): State<FileApiState>,
    Extension(rls): Extension<RlsContext>,
    Json(value): Json<FileType>,
) -> Response {
    let result = async {
        state.mutation(&rls, "types", "insert").await?;
        fdb_app::files::identifier(&value.type_id.0)?;
        if value.version < 1 || value.schema_profile != fdb_app::files::SCHEMA_PROFILE {
            return Err(FileError::Invalid);
        }
        state.store(&rls)?.put_type(&rls, &value).await
    }
    .await;
    response(result)
}
#[tracing::instrument(skip_all)]
pub(super) async fn match_type(
    State(state): State<FileApiState>,
    Extension(rls): Extension<RlsContext>,
    Json(input): Json<MatchInput>,
) -> Response {
    let result = async { fdb_app::files::match_type(state.store(&rls)?, &rls, &input).await }.await;
    response(result)
}
#[tracing::instrument(skip_all)]
pub(super) async fn get(
    State(state): State<FileApiState>,
    Extension(rls): Extension<RlsContext>,
    Path(id): Path<String>,
) -> Response {
    let result = async {
        fdb_app::files::identifier(&id)?;
        state.store(&rls)?.get(&rls, &FileId(id)).await
    }
    .await;
    response(result)
}
#[tracing::instrument(skip_all)]
pub(super) async fn put(
    State(state): State<FileApiState>,
    Extension(rls): Extension<RlsContext>,
    Path(id): Path<String>,
    Json(value): Json<PutFile>,
) -> Response {
    let result = async {
        let id = FileId(id);
        state
            .mutation(
                &rls,
                "records",
                if value.expected_revision == 0 {
                    "insert"
                } else {
                    "update"
                },
            )
            .await?;
        fdb_app::files::validate_put(&id, &value)?;
        state.store(&rls)?.put(&rls, &id, &value).await
    }
    .await;
    response(result)
}
#[tracing::instrument(skip_all)]
pub(super) async fn query(
    State(state): State<FileApiState>,
    Extension(rls): Extension<RlsContext>,
    Json(value): Json<FileQuery>,
) -> Response {
    let result = async {
        fdb_app::files::validate_query(&value)?;
        state.store(&rls)?.query(&rls, &value).await
    }
    .await;
    response(result)
}
