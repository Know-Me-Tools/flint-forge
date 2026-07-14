use fdb_domain::{ChangeEvent, ChangeOp};

/// Wire shape of a `flint_change` NOTIFY payload. `tenant`/`truncated` are consumed
/// and dropped — they are not part of the domain `ChangeEvent`.
#[derive(serde::Deserialize)]
struct RawNotify {
    op: String,
    schema: String,
    table: String,
    #[serde(default)]
    tenant: Option<String>,
    #[serde(default)]
    record: Option<serde_json::Value>,
    #[serde(default)]
    old_record: Option<serde_json::Value>,
    #[serde(default)]
    #[allow(dead_code)] // retained on the wire for operator debugging; not forwarded.
    truncated: bool,
}

/// Module-private payload error. NOT part of the public API and never reaches the
/// port (`StreamError` has only `Unavailable`/`Denied`); the listen loop drops on it.
#[derive(Debug, thiserror::Error)]
pub(super) enum PayloadError {
    #[error("payload json")]
    Json,
    #[error("unknown op")]
    UnknownOp,
}

/// Parse a NOTIFY payload into `(tenant, ChangeEvent)`. Pure — the core of the no-DB
/// unit tests. `tenant` is returned alongside for fan-out pre-filtering and is NOT
/// part of the domain `ChangeEvent`.
pub(super) fn parse_payload(raw: &str) -> Result<(Option<String>, ChangeEvent), PayloadError> {
    let parsed: RawNotify = serde_json::from_str(raw).map_err(|_| PayloadError::Json)?;
    let op = op_from_str(&parsed.op).ok_or(PayloadError::UnknownOp)?;
    let event = ChangeEvent {
        op,
        schema: parsed.schema,
        table: parsed.table,
        record: parsed.record,
        old_record: parsed.old_record,
    };
    Ok((parsed.tenant, event))
}

/// Map a lowercase wire op string to `ChangeOp`. The trigger emits lowercase, so
/// uppercase / unknown values map to `None` (and become `PayloadError::UnknownOp`).
pub(super) fn op_from_str(s: &str) -> Option<ChangeOp> {
    match s {
        "insert" => Some(ChangeOp::Insert),
        "update" => Some(ChangeOp::Update),
        "delete" => Some(ChangeOp::Delete),
        "upsert" => Some(ChangeOp::Upsert),
        _ => None,
    }
}
