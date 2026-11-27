//! Versioned file metadata HTTP interface; authenticated middleware is required.
mod handlers;
use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use fdb_ports::{
    files::{FileError, FileMetadataStore},
    KetoCheck,
};
use forge_identity::RlsContext;
use forge_policy::{Decision, Pep, Request};
use serde_json::{json, Value};
use std::sync::Arc;

/// Dependencies injected only by the composition root.
#[derive(Clone)]
pub struct FileApiState {
    /// Disabled means no file metadata pool is constructed.
    pub store: Option<Arc<dyn FileMetadataStore>>,
    /// Mandatory fail-closed mutation capability gate.
    pub pep: Arc<dyn Pep>,
    /// Existing optional coarse relationship gate, matching deployment mode.
    pub keto: Option<Arc<dyn KetoCheck>>,
}
impl FileApiState {
    fn store(&self, rls: &RlsContext) -> Result<&dyn FileMetadataStore, FileError> {
        fdb_app::files::owner(rls)?;
        self.store.as_deref().ok_or(FileError::Unavailable)
    }
    async fn mutation(&self, rls: &RlsContext, table: &str, action: &str) -> Result<(), FileError> {
        self.store(rls)?;
        let object = format!("flint_files.{table}");
        if let Some(keto) = &self.keto {
            if !keto
                .check("entities", &object, action, &rls.keto_subject)
                .await
            {
                return Err(FileError::Denied);
            }
        }
        let request = Request {
            action: action.to_owned(),
            resource: object,
            context: Value::Null,
        };
        if self.pep.check(rls, &request).await != Decision::Allow {
            return Err(FileError::Denied);
        }
        Ok(())
    }
}

/// Mounted even when disabled. Caller MUST layer verified JWT extraction.
#[must_use]
pub fn router(state: FileApiState) -> Router {
    Router::new()
        .route("/files/v1/capabilities", get(handlers::capabilities))
        .route(
            "/files/v1/types",
            get(handlers::types).put(handlers::put_type),
        )
        .route("/files/v1/types/match", post(handlers::match_type))
        .route("/files/v1/records/query", post(handlers::query))
        .route(
            "/files/v1/records/{file_id}",
            get(handlers::get).put(handlers::put),
        )
        .layer(DefaultBodyLimit::max(65_536))
        .with_state(state)
}

pub(super) fn error(error: FileError) -> axum::response::Response {
    use axum::{http::StatusCode, response::IntoResponse, Json};
    let (status, code) = match error {
        FileError::Invalid => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_metadata"),
        FileError::Conflict => (StatusCode::CONFLICT, "metadata_conflict"),
        FileError::NotFound => (StatusCode::NOT_FOUND, "metadata_not_found"),
        FileError::Ambiguous => (StatusCode::CONFLICT, "ambiguous_type"),
        FileError::Denied => (StatusCode::FORBIDDEN, "metadata_denied"),
        _ => (StatusCode::SERVICE_UNAVAILABLE, "metadata_unavailable"),
    };
    (status, Json(json!({"error":{"code":code}}))).into_response()
}

/// Parse explicit default-off availability; never silently enable on a typo.
/// # Errors
/// Rejects any value other than absent, false, or true.
pub fn enabled(value: Option<&str>) -> Result<bool, &'static str> {
    match value {
        None | Some("false") => Ok(false),
        Some("true") => Ok(true),
        _ => Err("FLINT_FILE_METADATA_ENABLED must be true or false"),
    }
}
