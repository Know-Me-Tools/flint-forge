//! REST `/rpc/<schema>/<fn>` handler — calls a Postgres function, with optional
//! `pgvector::Vector` argument binding. Split out of `rest/mod.rs` to keep files
//! under the 500-line limit.

use axum::{http::StatusCode, response::IntoResponse, Json};
use forge_domain::is_safe_identifier;
use serde_json::{json, Map, Value};
use tracing::instrument;

use super::RestState;
use crate::model::{is_vector_type, FnMeta};

/// Pick the overload whose argument names are an exact set-match for the
/// request body's keys; fall back to the first candidate when no exact match
/// exists, so a caller not supplying every arg name still resolves
/// deterministically rather than 404ing. Postgres allows function
/// overloading (same schema+name, different argument lists — e.g.
/// `cron.schedule(text, text)` vs `cron.schedule(text, text, text)`), so
/// `candidates` may contain more than one entry for the same (schema, name).
fn select_overload<'a>(candidates: &[&'a FnMeta], body: &Map<String, Value>) -> Option<&'a FnMeta> {
    candidates
        .iter()
        .find(|f| f.args.len() == body.len() && f.args.iter().all(|a| body.contains_key(&a.name)))
        .or_else(|| candidates.first())
        .copied()
}

/// POST /rpc/<schema>/<fn_name> — call a Postgres function with optional vector args.
///
/// Accepts a JSON object body where keys are arg names and values are arg values.
/// Args whose `pg_type` starts with `"vector"` are deserialized from `[f32, ...]`
/// JSON arrays into `pgvector::Vector` and bound with explicit `::vector` casting.
/// All other args are bound as generic JSONB.
///
/// Returns the result rows as a JSON array. Returns 400 on malformed vector args
/// or unknown function, 500 on database errors.
#[instrument(skip(state, body), fields(schema = %schema, fn_name = %fn_name))]
pub(super) async fn handle_rpc(
    schema: String,
    fn_name: String,
    state: RestState,
    body: Map<String, Value>,
) -> impl IntoResponse {
    // Locate the function metadata in the compiled model — see
    // `select_overload` for why more than one candidate can share (schema, name).
    let candidates: Vec<_> = state
        .model
        .functions
        .iter()
        .filter(|f| f.schema == schema && f.name == fn_name)
        .collect();
    let fn_meta = match select_overload(&candidates, &body) {
        Some(f) => f,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": format!("function {schema}.{fn_name} not found")})),
            )
                .into_response();
        }
    };

    // Build the parameterised SQL call:
    //   SELECT * FROM <schema>.<fn>($1, $2, ...) AS t
    // Placeholder numbering matches the arg order from FnMeta.
    let placeholders: Vec<String> = (1..=fn_meta.args.len()).map(|i| format!("${i}")).collect();
    // SECURITY: interpolate the identifiers from the *compiled model*
    // (`fn_meta`), not the raw path params — the model is reflected from
    // pg_catalog and is the trusted source. Belt-and-braces, re-validate.
    if !is_safe_identifier(&fn_meta.schema) || !is_safe_identifier(&fn_meta.name) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "invalid function identifier"})),
        )
            .into_response();
    }
    let call_sql = format!(
        "SELECT row_to_json(t) AS row FROM {}.{}({}) AS t",
        fn_meta.schema,
        fn_meta.name,
        placeholders.join(", ")
    );

    // Bind arguments in declaration order, dispatching on pg_type.
    // SAFETY: `fn_meta.schema`/`fn_meta.name` passed `is_safe_identifier` above.
    let mut q = sqlx::query(sqlx::AssertSqlSafe(call_sql));
    for arg in &fn_meta.args {
        let val = body.get(&arg.name).cloned().unwrap_or(Value::Null);
        if is_vector_type(&arg.pg_type) {
            match json_to_vector(val) {
                Ok(vec) => {
                    q = q.bind(vec);
                }
                Err(msg) => {
                    return (StatusCode::BAD_REQUEST, Json(json!({"error": msg}))).into_response();
                }
            }
        } else {
            q = q.bind(val);
        }
    }

    // Execute and collect result rows.
    match q.fetch_all(&state.pool).await {
        Ok(rows) => {
            let result: Vec<Value> = rows
                .into_iter()
                .filter_map(|row| {
                    use sqlx::Row;
                    row.try_get::<Value, _>("row").ok()
                })
                .collect();
            Json(Value::Array(result)).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, schema = %schema, fn_name = %fn_name, "rpc execution error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal server error"})),
            )
                .into_response()
        }
    }
}

/// Deserialize a JSON value as a `pgvector::Vector`.
///
/// Expects a JSON array of numbers. Returns a descriptive error string on mismatch.
fn json_to_vector(val: Value) -> Result<pgvector::Vector, String> {
    let arr = match val {
        Value::Array(a) => a,
        Value::Null => return Err("vector arg must be a JSON array, got null".into()),
        other => {
            return Err(format!(
                "vector arg must be a JSON array, got {}",
                other.type_name_for_error()
            ));
        }
    };

    let floats: Result<Vec<f32>, _> = arr
        .into_iter()
        .map(|v| {
            v.as_f64()
                .map(|f| f as f32)
                .ok_or_else(|| "vector elements must be numbers".to_owned())
        })
        .collect();

    Ok(pgvector::Vector::from(floats?))
}

/// Helper to name the JSON type in error messages.
trait JsonTypeName {
    fn type_name_for_error(&self) -> &'static str;
}

impl JsonTypeName for Value {
    fn type_name_for_error(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{json_to_vector, select_overload};
    use crate::model::ArgMeta;
    use serde_json::json;

    fn fn_meta(name: &str, arg_names: &[&str]) -> super::FnMeta {
        super::FnMeta {
            schema: "cron".into(),
            name: name.into(),
            args: arg_names
                .iter()
                .map(|n| ArgMeta {
                    name: (*n).into(),
                    pg_type: "text".into(),
                })
                .collect(),
            return_type: "bigint".into(),
            security_definer: false,
        }
    }

    /// p16-c-followup gate: given two overloads, the one whose argument names
    /// exactly match the request body's keys must be selected — not
    /// whichever happens to be first in the model.
    #[test]
    fn select_overload_picks_exact_arg_match() {
        let two_arg = fn_meta("schedule", &["schedule", "command"]);
        let three_arg = fn_meta("schedule", &["job_name", "schedule", "command"]);
        let candidates = vec![&two_arg, &three_arg];

        let body: serde_json::Map<String, serde_json::Value> = [
            ("job_name".to_string(), json!("nightly")),
            ("schedule".to_string(), json!("0 3 * * *")),
            ("command".to_string(), json!("SELECT 1;")),
        ]
        .into_iter()
        .collect();

        let selected = select_overload(&candidates, &body).expect("a candidate matches");
        assert_eq!(selected.args.len(), 3, "must select the 3-arg overload");
    }

    #[test]
    fn select_overload_picks_the_other_exact_match() {
        let two_arg = fn_meta("schedule", &["schedule", "command"]);
        let three_arg = fn_meta("schedule", &["job_name", "schedule", "command"]);
        let candidates = vec![&two_arg, &three_arg];

        let body: serde_json::Map<String, serde_json::Value> = [
            ("schedule".to_string(), json!("0 3 * * *")),
            ("command".to_string(), json!("SELECT 1;")),
        ]
        .into_iter()
        .collect();

        let selected = select_overload(&candidates, &body).expect("a candidate matches");
        assert_eq!(selected.args.len(), 2, "must select the 2-arg overload");
    }

    #[test]
    fn select_overload_falls_back_to_first_when_no_exact_match() {
        let two_arg = fn_meta("schedule", &["schedule", "command"]);
        let candidates = vec![&two_arg];

        // Body doesn't match any overload's arg set exactly (extra key).
        let body: serde_json::Map<String, serde_json::Value> = [
            ("schedule".to_string(), json!("0 3 * * *")),
            ("command".to_string(), json!("SELECT 1;")),
            ("unexpected".to_string(), json!("value")),
        ]
        .into_iter()
        .collect();

        let selected = select_overload(&candidates, &body).expect("falls back to first");
        assert_eq!(selected.args.len(), 2);
    }

    #[test]
    fn select_overload_returns_none_for_empty_candidates() {
        let candidates: Vec<&super::FnMeta> = vec![];
        let body = serde_json::Map::new();
        assert!(select_overload(&candidates, &body).is_none());
    }

    #[test]
    fn json_to_vector_accepts_float_array() {
        let v = json!([0.1_f64, 0.2_f64, 0.3_f64]);
        let vec = json_to_vector(v).expect("should parse");
        let floats: Vec<f32> = vec.into();
        assert_eq!(floats.len(), 3);
        assert!((floats[0] - 0.1_f32).abs() < 1e-6);
    }

    #[test]
    fn json_to_vector_rejects_non_array() {
        assert!(json_to_vector(json!("not an array")).is_err());
        assert!(json_to_vector(json!(42)).is_err());
        assert!(json_to_vector(serde_json::Value::Null).is_err());
    }

    #[test]
    fn json_to_vector_rejects_non_numeric_elements() {
        assert!(json_to_vector(json!(["a", "b"])).is_err());
    }
}
