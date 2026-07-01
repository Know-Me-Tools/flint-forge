use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
};
use serde_json::{Map, Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::instrument;

use crate::model::{DatabaseModel, is_vector_type};
use crate::passes::endpoint_generation::{EndpointKind, generate};

/// Shared state threaded into every route handler.
#[derive(Clone)]
struct RestState {
    model: Arc<DatabaseModel>,
    pool: PgPool,
}

/// Compiles a `DatabaseModel` into an Axum `Router` with CRUD + RPC handlers.
///
/// The resulting router exposes:
/// - `GET  /<schema>/<table>`       — list rows
/// - `POST /<schema>/<table>`       — insert row
/// - `PATCH /<schema>/<table>/:id`  — update row
/// - `DELETE /<schema>/<table>/:id` — delete row
/// - `POST /rpc/<schema>/<fn>`      — call stored function (vector args supported)
///
/// CRUD handlers remain `todo!()` stubs pending the query-builder landing.
/// `handle_rpc` is fully implemented: it detects `vector(N)` arg types and binds
/// `pgvector::Vector` typed parameters automatically.
pub struct RestCompiler;

impl RestCompiler {
    pub fn compile(model: &DatabaseModel, pool: PgPool) -> Router {
        let state = RestState {
            model: Arc::new(model.clone()),
            pool,
        };

        let endpoints = generate(model);

        let mut router: Router<RestState> = Router::new();

        for endpoint in &endpoints {
            let path = endpoint.path.clone();
            router = match (&endpoint.kind, endpoint.method) {
                (EndpointKind::TableList { .. }, "GET") => {
                    router.route(&path, get(handle_list))
                }
                (EndpointKind::TableList { .. }, "POST") => {
                    router.route(&path, post(handle_insert))
                }
                (EndpointKind::TableById { .. }, "PATCH") => {
                    router.route(&path, patch(handle_update))
                }
                (EndpointKind::TableById { .. }, "DELETE") => {
                    router.route(&path, delete(handle_delete))
                }
                (EndpointKind::Rpc { .. }, "POST") => {
                    router.route(&path, post(handle_rpc))
                }
                _ => router,
            };
        }

        router.with_state(state)
    }
}

async fn handle_list(State(_state): State<RestState>) -> StatusCode {
    todo!("p2-c004: list rows via REST query builder")
}

async fn handle_insert(State(_state): State<RestState>) -> StatusCode {
    todo!("p2-c004: insert row via REST query builder")
}

async fn handle_update(
    State(_state): State<RestState>,
    Path(_id): Path<String>,
) -> StatusCode {
    todo!("p2-c004: update row via REST query builder")
}

async fn handle_delete(
    State(_state): State<RestState>,
    Path(_id): Path<String>,
) -> StatusCode {
    todo!("p2-c004: delete row via REST query builder")
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
async fn handle_rpc(
    State(state): State<RestState>,
    Path((schema, fn_name)): Path<(String, String)>,
    Json(body): Json<Map<String, Value>>,
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
    let placeholders: Vec<String> = (1..=fn_meta.args.len())
        .map(|i| format!("${i}"))
        .collect();
    let call_sql = format!(
        "SELECT row_to_json(t) AS row FROM {schema}.{fn_name}({}) AS t",
        placeholders.join(", ")
    );

    // Bind arguments in declaration order, dispatching on pg_type.
    let mut q = sqlx::query(&call_sql);
    for arg in &fn_meta.args {
        let val = body.get(&arg.name).cloned().unwrap_or(Value::Null);
        if is_vector_type(&arg.pg_type) {
            match json_to_vector(val) {
                Ok(vec) => {
                    q = q.bind(vec);
                }
                Err(msg) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"error": msg})),
                    )
                        .into_response();
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
    use crate::model::{DatabaseModel, Table};
    use super::RestCompiler;

    fn minimal_model() -> DatabaseModel {
        DatabaseModel {
            tables: vec![Table {
                schema: "public".into(),
                name: "items".into(),
                columns: vec![],
                pk: vec![],
                fk: vec![],
                rls_enabled: true,
                vault_key: None,
            }],
            functions: vec![],
            views: vec![],
            version: 1,
        }
    }

    #[tokio::test]
    async fn compiles_without_panic_for_minimal_model() {
        // compile() must not panic during route registration.
        // Uses a disconnected lazy pool — compilation never touches the DB.
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test")
            .expect("lazy pool");
        let model = minimal_model();
        let _router = RestCompiler::compile(&model, pool);
    }

    #[test]
    fn json_to_vector_accepts_float_array() {
        use serde_json::json;
        use super::json_to_vector;
        let v = json!([0.1_f64, 0.2_f64, 0.3_f64]);
        let vec = json_to_vector(v).expect("should parse");
        let floats: Vec<f32> = vec.into();
        assert_eq!(floats.len(), 3);
        assert!((floats[0] - 0.1_f32).abs() < 1e-6);
    }

    #[test]
    fn json_to_vector_rejects_non_array() {
        use serde_json::json;
        use super::json_to_vector;
        assert!(json_to_vector(json!("not an array")).is_err());
        assert!(json_to_vector(json!(42)).is_err());
        assert!(json_to_vector(serde_json::Value::Null).is_err());
    }

    #[test]
    fn json_to_vector_rejects_non_numeric_elements() {
        use serde_json::json;
        use super::json_to_vector;
        assert!(json_to_vector(json!(["a", "b"])).is_err());
    }
}
