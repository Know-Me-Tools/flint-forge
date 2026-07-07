//! SPI writeback helpers for the Flint Ember background worker.

use pgrx::bgworkers::BackgroundWorker;
use pgrx::datum::DatumWithOid;
use pgrx::prelude::*;
use pgrx::JsonB;
use serde_json::Value;

/// Write an embedding vector into a user table row using the safe SQL helper
/// `llm.writeback_vector`.
pub fn write_vector(
    schema: &str,
    table: &str,
    pk: Option<&Value>,
    column: &str,
    vector: &[f32],
    dimensions: i32,
) -> Result<(), pgrx::spi::Error> {
    let pk_json = pk.cloned().unwrap_or(Value::Null);
    let vector_text = format!(
        "[{}]",
        vector
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );

    BackgroundWorker::transaction(|| {
        let sql = "SELECT llm.writeback_vector($1, $2, $3, $4, $5, $6)";
        let args: &[DatumWithOid<'_>] = &[
            DatumWithOid::from(schema),
            DatumWithOid::from(table),
            DatumWithOid::from(JsonB(pk_json)),
            DatumWithOid::from(column),
            DatumWithOid::from(vector_text.as_str()),
            DatumWithOid::from(dimensions),
        ];
        Spi::run_with_args(sql, args)
    })
}

/// Write a plain text result into a user table row using the safe SQL helper
/// `llm.writeback_text`.
pub fn write_text(
    schema: &str,
    table: &str,
    pk: Option<&Value>,
    column: &str,
    text: &str,
) -> Result<(), pgrx::spi::Error> {
    let pk_json = pk.cloned().unwrap_or(Value::Null);

    BackgroundWorker::transaction(|| {
        let sql = "SELECT llm.writeback_text($1, $2, $3, $4, $5)";
        let args: &[DatumWithOid<'_>] = &[
            DatumWithOid::from(schema),
            DatumWithOid::from(table),
            DatumWithOid::from(JsonB(pk_json)),
            DatumWithOid::from(column),
            DatumWithOid::from(text),
        ];
        Spi::run_with_args(sql, args)
    })
}
