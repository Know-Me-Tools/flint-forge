//! `flint:host/llm` — governed inference via the same governed SQL path as
//! `flint:host/db`, calling the `llm.embed`/`llm.complete` SQL functions
//! (`ext-flint-llm`), which route through flint-gate. The component never
//! holds a provider key either way — that was already true at the SQL layer.

use crate::host_bindings::flint::host::llm::{Host, HostError};
use crate::KilnHostState;
use fke_domain::Capability;
use forge_identity::RlsContext;

fn json_string(s: &str) -> String {
    serde_json::to_string(s).expect("string JSON-encoding is infallible")
}

fn internal_error(context: &str, e: impl std::fmt::Display) -> HostError {
    HostError {
        code: "QUERY_FAILED".to_owned(),
        message: format!("{context}: {e}"),
    }
}

/// Run `sql` (a single `SELECT ... AS result_value` expression) via
/// `query_json` and pull the `result_value` field out of the single
/// returned row.
async fn call_scalar(
    database: &dyn fdb_ports::DatabaseBackend,
    rls: &RlsContext,
    sql: &str,
    params: &[String],
) -> Result<serde_json::Value, HostError> {
    let rows = database
        .query_json(rls, sql, params)
        .await
        .map_err(|e| internal_error("llm call", e))?;
    let row_json = rows
        .first()
        .ok_or_else(|| internal_error("llm call", "SQL function returned no row"))?;
    let row: serde_json::Value =
        serde_json::from_str(row_json).map_err(|e| internal_error("decode row", e))?;
    row.get("result_value")
        .cloned()
        .ok_or_else(|| internal_error("decode row", "missing result_value column"))
}

impl Host for KilnHostState {
    /// `SELECT llm.embed($1, $2) AS result_value` — `model` falls back to
    /// the SQL function's own `DEFAULT 'default'` when `None`.
    async fn embed(&mut self, input: String, model: Option<String>) -> Result<Vec<f32>, HostError> {
        if !self.granted.contains(&Capability::Llm) {
            return Err(HostError {
                code: "CAPABILITY_DENIED".to_owned(),
                message: "Llm capability not granted for this invocation".to_owned(),
            });
        }
        let database = self.database.as_ref().ok_or_else(|| HostError {
            code: "UNAVAILABLE".to_owned(),
            message: "no database backend configured for this Kiln runtime".to_owned(),
        })?;
        let rls = self.identity.as_ref().ok_or_else(|| HostError {
            code: "NO_IDENTITY".to_owned(),
            message: "no caller identity established for this invocation".to_owned(),
        })?;

        let (sql, params) = match &model {
            Some(m) => (
                "SELECT llm.embed($1, $2) AS result_value",
                vec![json_string(&input), json_string(m)],
            ),
            None => (
                "SELECT llm.embed($1) AS result_value",
                vec![json_string(&input)],
            ),
        };
        let value = call_scalar(database.as_ref(), rls, sql, &params).await?;
        serde_json::from_value(value).map_err(|e| internal_error("decode embedding", e))
    }

    /// `SELECT llm.complete($1, $2::jsonb, $3) AS result_value` — `opts` is
    /// already a JSON object per the WIT contract; a `"model"` field inside
    /// it (if present) is additionally forwarded as the SQL function's
    /// separate positional `model` argument, since `llm.complete` reads
    /// model from that argument, not from `opts` itself.
    async fn complete(&mut self, prompt: String, opts: String) -> Result<String, HostError> {
        if !self.granted.contains(&Capability::Llm) {
            return Err(HostError {
                code: "CAPABILITY_DENIED".to_owned(),
                message: "Llm capability not granted for this invocation".to_owned(),
            });
        }
        let database = self.database.as_ref().ok_or_else(|| HostError {
            code: "UNAVAILABLE".to_owned(),
            message: "no database backend configured for this Kiln runtime".to_owned(),
        })?;
        let rls = self.identity.as_ref().ok_or_else(|| HostError {
            code: "NO_IDENTITY".to_owned(),
            message: "no caller identity established for this invocation".to_owned(),
        })?;

        let model: Option<String> = serde_json::from_str::<serde_json::Value>(&opts)
            .ok()
            .and_then(|v| v.get("model").and_then(|m| m.as_str()).map(str::to_owned));

        let (sql, params) = match &model {
            Some(m) => (
                "SELECT llm.complete($1, $2::jsonb, $3) AS result_value",
                vec![json_string(&prompt), opts.clone(), json_string(m)],
            ),
            None => (
                "SELECT llm.complete($1, $2::jsonb) AS result_value",
                vec![json_string(&prompt), opts.clone()],
            ),
        };
        let value = call_scalar(database.as_ref(), rls, sql, &params).await?;
        value
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| internal_error("decode completion", "result_value was not a string"))
    }
}
