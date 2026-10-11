use axum::{
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use fdb_ports::IdempotencyContext;
use forge_identity::RlsContext;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

use super::{bad_request, RestState};

pub(super) async fn execute_mutation(
    state: &RestState,
    sql: &str,
    params: Vec<fdb_query::QueryParam>,
    rls: &RlsContext,
    idempotency: Option<&IdempotencyContext>,
) -> Result<Vec<Map<String, Value>>, fdb_ports::BackendError> {
    match idempotency {
        Some(context) => {
            state
                .executor
                .execute_idempotent(sql, params, rls, context)
                .await
        }
        None => state.executor.execute_raw(sql, params, rls).await,
    }
}

pub(super) fn idempotency_context(
    headers: &HeaderMap,
    action: &str,
    schema: &str,
    table: &str,
    body: Option<&Map<String, Value>>,
    query: Option<&HashMap<String, String>>,
) -> Result<Option<IdempotencyContext>, axum::response::Response> {
    let Some(raw_key) = headers.get("idempotency-key") else {
        return Ok(None);
    };
    let key = raw_key
        .to_str()
        .map_err(|_| bad_request("invalid Idempotency-Key header"))?;
    if key.is_empty()
        || key.len() > 128
        || !key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(bad_request("invalid Idempotency-Key header"));
    }

    let mut hasher = Sha256::new();
    for part in [action, schema, table] {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    if let Some(body) = body {
        let mut entries: Vec<_> = body.iter().collect();
        entries.sort_unstable_by(|left, right| left.0.cmp(right.0));
        for (name, value) in entries {
            hasher.update(name.as_bytes());
            hasher.update([0]);
            hasher.update(value.to_string().as_bytes());
            hasher.update([0]);
        }
    }
    if let Some(query) = query {
        let mut entries: Vec<_> = query.iter().collect();
        entries.sort_unstable_by(|left, right| left.0.cmp(right.0));
        for (name, value) in entries {
            hasher.update(name.as_bytes());
            hasher.update([0]);
            hasher.update(value.as_bytes());
            hasher.update([0]);
        }
    }
    Ok(Some(IdempotencyContext {
        mutation_id: key.to_owned(),
        request_fingerprint: format!("{:x}", hasher.finalize()),
    }))
}

pub(super) fn idempotency_conflict() -> axum::response::Response {
    (
        StatusCode::CONFLICT,
        Json(serde_json::json!({
            "error": "idempotency key was already used for a different mutation"
        })),
    )
        .into_response()
}
