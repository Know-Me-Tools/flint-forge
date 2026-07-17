//! Pure helper functions for surface assembly.

use serde_json::Value;

use super::error::AssemblerError;
use super::types::AssemblyContext;

/// Build the canonical catalog id for a context.
pub(super) fn catalog_id_for(ctx: &AssemblyContext) -> String {
    ctx.event_payload
        .get("catalog_id")
        .and_then(Value::as_str)
        .map_or_else(
            || "https://forge.example.com/a2ui/v1/catalog/flint-base/1.0.0".to_string(),
            ToOwned::to_owned,
        )
}

/// Extract `data_source.schema` / `data_source.table` from the event payload.
pub(super) fn data_source(ctx: &AssemblyContext) -> Result<(String, String), AssemblerError> {
    let ds = ctx
        .event_payload
        .get("data_source")
        .ok_or_else(|| AssemblerError::MissingField("data_source".to_string()))?;

    let schema = ds
        .get("schema")
        .and_then(Value::as_str)
        .unwrap_or("public")
        .to_string();
    let table = ds
        .get("table")
        .and_then(Value::as_str)
        .ok_or_else(|| AssemblerError::MissingField("data_source.table".to_string()))?
        .to_string();

    Ok((schema, table))
}

/// Check whether `payload` satisfies all predicates in `filter`.
///
/// Filter keys may be dotted paths (e.g. `data_source.table`). An empty filter
/// object matches every payload.
pub(super) fn matches_filter(payload: &Value, filter: &Value) -> bool {
    let Some(predicates) = filter.as_object() else {
        return true;
    };

    for (key, expected) in predicates {
        let actual = navigate(payload, key);
        if !value_matches(actual, expected) {
            return false;
        }
    }

    true
}

/// Navigate a dotted path through a JSON object. Missing segments yield Null.
fn navigate<'v>(value: &'v Value, path: &str) -> &'v Value {
    let mut current = value;
    for segment in path.split('.') {
        current = current.get(segment).unwrap_or(&Value::Null);
        if current.is_null() {
            break;
        }
    }
    current
}

/// Compare an actual JSON value to an expected predicate value. Null expected
/// values are interpreted as "missing or null".
fn value_matches(actual: &Value, expected: &Value) -> bool {
    if expected.is_null() {
        return actual.is_null();
    }
    actual == expected
}
