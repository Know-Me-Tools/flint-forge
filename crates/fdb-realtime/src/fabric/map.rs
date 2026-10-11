use std::collections::HashSet;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use fdb_domain::{ChangeEvent, ChangeOp};
use fdb_ports::StreamError;
use frf_proto::fv2::{
    canonical_value, watch_entity_type_response, CanonicalValue, ClientCheckpoint, EntityMutation,
    EntityRecord, WatchEntityTypeResponse,
};
use prost::Message as _;
use serde_json::{Map, Number, Value};

#[derive(Clone)]
pub(super) struct ExpectedScope {
    pub(super) schema: String,
    pub(super) table: String,
    pub(super) projection: String,
    pub(super) tenant: String,
}

pub(super) fn map_response(
    response: WatchEntityTypeResponse,
    expected: &ExpectedScope,
) -> Result<Option<ChangeEvent>, StreamError> {
    match response.frame {
        Some(watch_entity_type_response::Frame::Accepted(accepted)) => {
            let entity_type = accepted.entity_type.ok_or(StreamError::InvalidData)?;
            scope_matches(
                &entity_type.schema,
                &entity_type.name,
                &entity_type.projection,
                &accepted.tenant_id,
                expected,
            )?;
            Ok(None)
        }
        Some(watch_entity_type_response::Frame::Mutation(mutation)) => {
            map_mutation(mutation, expected).map(Some)
        }
        Some(watch_entity_type_response::Frame::CheckpointAdvanced(_)) => Ok(None),
        Some(watch_entity_type_response::Frame::ResnapshotRequired(_)) => {
            Err(StreamError::ResnapshotRequired)
        }
        Some(watch_entity_type_response::Frame::Lagged(_)) => Err(StreamError::Lagged),
        Some(watch_entity_type_response::Frame::SnapshotRow(_))
        | Some(watch_entity_type_response::Frame::SnapshotComplete(_))
        | None => Err(StreamError::InvalidData),
    }
}

fn map_mutation(
    mutation: EntityMutation,
    expected: &ExpectedScope,
) -> Result<ChangeEvent, StreamError> {
    let op = match mutation.op() {
        frf_proto::fv2::ChangeOp::Insert => ChangeOp::Insert,
        frf_proto::fv2::ChangeOp::Update => ChangeOp::Update,
        frf_proto::fv2::ChangeOp::Delete => ChangeOp::Delete,
        frf_proto::fv2::ChangeOp::Upsert => ChangeOp::Upsert,
        _ => return Err(StreamError::InvalidData),
    };
    let entity_type = mutation.entity_type.ok_or(StreamError::InvalidData)?;
    scope_matches(
        &entity_type.schema,
        &entity_type.name,
        &entity_type.projection,
        &mutation.tenant_id,
        expected,
    )?;
    if mutation.event_id.is_empty() {
        return Err(StreamError::InvalidData);
    }
    let key = mutation.key.ok_or(StreamError::InvalidData)?;
    if key.parts.is_empty() {
        return Err(StreamError::InvalidData);
    }
    let mut key_object = Map::new();
    for part in key.parts {
        if key_object.contains_key(&part.column) || part.column.is_empty() {
            return Err(StreamError::InvalidData);
        }
        key_object.insert(
            part.column,
            canonical_to_json(part.value.ok_or(StreamError::InvalidData)?)?,
        );
    }

    let checkpoint = encode_checkpoint(
        mutation
            .checkpoint
            .as_ref()
            .ok_or(StreamError::InvalidData)?,
    );
    if op == ChangeOp::Delete {
        if mutation.record.is_some() {
            return Err(StreamError::InvalidData);
        }
        return Ok(ChangeEvent {
            op,
            schema: entity_type.schema,
            table: entity_type.name,
            record: None,
            old_record: Some(Value::Object(key_object)),
            event_id: Some(mutation.event_id),
            checkpoint: Some(checkpoint),
            projection: Some(Vec::new()),
            delete_authorized: true,
            truncated: false,
        });
    }

    let record = mutation.record.ok_or(StreamError::InvalidData)?;
    let (record, projection) = record_to_json(record)?;
    for (column, value) in key_object {
        match record.get(&column) {
            Some(existing) if existing == &value => {}
            Some(_) | None => return Err(StreamError::InvalidData),
        }
    }
    Ok(ChangeEvent {
        op,
        schema: entity_type.schema,
        table: entity_type.name,
        record: Some(Value::Object(record)),
        old_record: None,
        event_id: Some(mutation.event_id),
        checkpoint: Some(checkpoint),
        projection: Some(projection),
        delete_authorized: false,
        truncated: false,
    })
}

fn record_to_json(record: EntityRecord) -> Result<(Map<String, Value>, Vec<String>), StreamError> {
    let mut names = HashSet::with_capacity(record.fields.len());
    let mut object = Map::new();
    let mut projection = Vec::with_capacity(record.fields.len());
    for field in record.fields {
        if field.column.is_empty() || !names.insert(field.column.clone()) {
            return Err(StreamError::InvalidData);
        }
        projection.push(field.column.clone());
        object.insert(
            field.column,
            canonical_to_json(field.value.ok_or(StreamError::InvalidData)?)?,
        );
    }
    Ok((object, projection))
}

fn canonical_to_json(value: CanonicalValue) -> Result<Value, StreamError> {
    let value = match value.kind.ok_or(StreamError::InvalidData)? {
        canonical_value::Kind::NullValue(_) => Value::Null,
        canonical_value::Kind::BoolValue(value) => Value::Bool(value),
        canonical_value::Kind::SignedInteger(value) => Value::Number(value.into()),
        canonical_value::Kind::UnsignedInteger(value) => Value::Number(value.into()),
        canonical_value::Kind::FloatValue(value) => {
            Value::Number(Number::from_f64(value).ok_or(StreamError::InvalidData)?)
        }
        canonical_value::Kind::TextValue(value)
        | canonical_value::Kind::DecimalValue(value)
        | canonical_value::Kind::UuidValue(value)
        | canonical_value::Kind::DateValue(value)
        | canonical_value::Kind::TimeValue(value) => Value::String(value),
        canonical_value::Kind::BytesValue(value) => Value::String(URL_SAFE_NO_PAD.encode(value)),
        canonical_value::Kind::TimestampValue(value) => {
            let nanos = u32::try_from(value.nanos).map_err(|_| StreamError::InvalidData)?;
            let timestamp = chrono::DateTime::from_timestamp(value.seconds, nanos)
                .ok_or(StreamError::InvalidData)?;
            Value::String(timestamp.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true))
        }
        canonical_value::Kind::JsonValue(value) => {
            serde_json::from_slice(&value).map_err(|_| StreamError::InvalidData)?
        }
    };
    Ok(value)
}

fn scope_matches(
    schema: &str,
    table: &str,
    projection: &str,
    tenant: &str,
    expected: &ExpectedScope,
) -> Result<(), StreamError> {
    if schema == expected.schema
        && table == expected.table
        && projection == expected.projection
        && tenant == expected.tenant
    {
        Ok(())
    } else {
        Err(StreamError::InvalidData)
    }
}

pub(super) fn encode_checkpoint(checkpoint: &ClientCheckpoint) -> String {
    URL_SAFE_NO_PAD.encode(checkpoint.encode_to_vec())
}

pub(super) fn decode_checkpoint(encoded: &str) -> Result<ClientCheckpoint, StreamError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| StreamError::InvalidCheckpoint)?;
    ClientCheckpoint::decode(bytes.as_slice()).map_err(|_| StreamError::InvalidCheckpoint)
}
