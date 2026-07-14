//! REST `/rpc/<schema>/<fn>` handler — calls a Postgres function, with optional
//! `pgvector::Vector` argument binding. Split out of `rest/mod.rs` to keep files
//! under the 500-line limit.

use axum::{http::StatusCode, response::IntoResponse, Json};
use forge_domain::is_safe_identifier;
use serde_json::{json, Map, Value};
use tracing::instrument;

use super::RestState;
use crate::model::is_vector_type;

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
    // Locate the function metadata in the compiled model.
    let fn_meta = match state
        .model
        .functions
        .iter()
        .find(|f| f.schema == schema && f.name == fn_name)
    {
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
    use super::json_to_vector;
    use serde_json::json;

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
