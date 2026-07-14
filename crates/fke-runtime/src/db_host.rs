//! `flint:host/db` — governed SQL access via the same RLS-scoped connection
//! REST and GraphQL use (`fdb_ports::DatabaseBackend::query_json`).

use crate::host_bindings::flint::host::db::{Host, HostError};
use crate::KilnHostState;
use fke_domain::Capability;

impl Host for KilnHostState {
    /// `sql`/`params` are forwarded verbatim to `DatabaseBackend::query_json`
    /// under the caller's `RlsContext` — RLS is the authorization boundary,
    /// not this method. `params[i]` are JSON-encoded scalar bind values, one
    /// per `$N` placeholder, per the WIT contract.
    async fn query(&mut self, sql: String, params: Vec<String>) -> Result<Vec<String>, HostError> {
        if !self.granted.contains(&Capability::Db) {
            return Err(HostError {
                code: "CAPABILITY_DENIED".to_owned(),
                message: "Db capability not granted for this invocation".to_owned(),
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

        database
            .query_json(rls, &sql, &params)
            .await
            .map_err(|e| HostError {
                code: "QUERY_FAILED".to_owned(),
                message: e.to_string(),
            })
    }
}
