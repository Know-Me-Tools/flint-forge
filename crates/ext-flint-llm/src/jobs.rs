//! Async job queue model and dequeue helpers for the Flint Ember background worker.

use pgrx::bgworkers::BackgroundWorker;
use pgrx::prelude::*;
use pgrx::JsonB;
use serde_json::Value;

/// A row from `llm.jobs` waiting to be processed.
#[derive(Debug, Clone)]
pub struct JobRow {
    pub id: i64,
    pub kind: String,
    pub schema_name: Option<String>,
    pub table_name: Option<String>,
    pub pk: Option<Value>,
    pub source: Option<Value>,
    pub target_column: Option<String>,
    pub model: Option<String>,
    pub dimensions: i32,
    pub origin_jwt: Option<String>,
    pub retry_count: i32,
}

/// Dequeue up to `batch_size` pending-visible jobs using `FOR UPDATE SKIP LOCKED`.
pub fn dequeue_pending(batch_size: i64) -> Result<Vec<JobRow>, pgrx::spi::Error> {
    let sql = format!(
        "SELECT id, kind, schema_name, table_name, pk, source, target_column, model, dimensions, origin_jwt, retry_count
         FROM llm.jobs
         WHERE status = 'pending' AND visible_at <= now()
         ORDER BY id
         FOR UPDATE SKIP LOCKED
         LIMIT {batch_size}"
    );

    BackgroundWorker::transaction(|| {
        Spi::connect(|client| {
            let mut rows = Vec::new();
            let tupletable = client.select(&sql, None, &[])?;
            for row in tupletable {
                rows.push(JobRow {
                    id: row.get_datum_by_ordinal(1)?.value::<i64>()?.unwrap_or(0),
                    kind: row
                        .get_datum_by_ordinal(2)?
                        .value::<String>()?
                        .unwrap_or_default(),
                    schema_name: row.get_datum_by_ordinal(3)?.value::<String>()?,
                    table_name: row.get_datum_by_ordinal(4)?.value::<String>()?,
                    pk: row.get_datum_by_ordinal(5)?.value::<JsonB>()?.map(|j| j.0),
                    source: row.get_datum_by_ordinal(6)?.value::<JsonB>()?.map(|j| j.0),
                    target_column: row.get_datum_by_ordinal(7)?.value::<String>()?,
                    model: row.get_datum_by_ordinal(8)?.value::<String>()?,
                    dimensions: row.get_datum_by_ordinal(9)?.value::<i32>()?.unwrap_or(1536),
                    origin_jwt: row.get_datum_by_ordinal(10)?.value::<String>()?,
                    retry_count: row.get_datum_by_ordinal(11)?.value::<i32>()?.unwrap_or(0),
                });
            }
            Ok(rows)
        })
    })
}

/// Mark a single job as completed.
pub fn mark_completed(id: i64) -> Result<(), pgrx::spi::Error> {
    BackgroundWorker::transaction(|| {
        Spi::run(&format!(
            "UPDATE llm.jobs SET status = 'completed', visible_at = now() WHERE id = {id}"
        ))
    })
}

/// Mark a single job as failed. If `retry` is true, schedule it with exponential
/// backoff based on the current `retry_count`.
pub fn mark_failed(
    id: i64,
    retry_count: i32,
    retry: bool,
    message: &str,
) -> Result<(), pgrx::spi::Error> {
    if retry {
        let backoff_minutes = 2_i32.pow(retry_count.min(4) as u32);
        BackgroundWorker::transaction(|| {
            Spi::run(&format!(
                "UPDATE llm.jobs
                 SET status = 'pending',
                     retry_count = retry_count + 1,
                     visible_at = now() + interval '{backoff_minutes} minutes'
                 WHERE id = {id}"
            ))
        })
    } else {
        let escaped = message.replace('\'', "''");
        BackgroundWorker::transaction(|| {
            Spi::run(&format!(
                "UPDATE llm.jobs
                 SET status = 'failed',
                     visible_at = now(),
                     source = COALESCE(source, '{{}}'::jsonb) || jsonb_build_object('error', '{escaped}')
                 WHERE id = {id}"
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_escape_helper() {
        let pk = serde_json::json!({"id": "it's"});
        let s = serde_json::to_string(&pk).unwrap().replace('\'', "''");
        assert!(!s.contains('\''));
    }
}
