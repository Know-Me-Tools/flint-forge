//! `flint:host/db` — governed SQL access.
//!
//! The WIT `db` interface exposes a single function
//! `query(sql, params) -> result<list<string>, host-error>`. The host routes
//! every query through `flint-gate` under the component's verified origin
//! JWT, so a component never holds a raw database credential.
//!
//! Skill authors do not call the WIT import directly. Instead, they implement
//! [`Database`] as a one-line adapter over their `bindings::flint::host::db`
//! module (see the crate README for a complete example), then call
//! [`Database::query`] which returns fully-decoded [`DbRow`]s and a typed
//! [`SkillError`] on failure.

use crate::error::SkillResult;
use crate::types::DbRow;
use std::future::Future;

/// Governed SQL access for Flint skills.
///
/// Implement this trait as a thin adapter over the WIT-generated
/// `bindings::flint::host::db` module. A typical implementation is:
///
/// ```no_run
/// # use flint_skill::{Database, SkillResult, SkillError, DbRow, HostInterface};
/// # use std::future::Future;
/// # struct Db;
/// # impl flint_skill::Database for Db {
/// fn query(&self, sql: &str, params: &[String]) -> impl Future<Output = SkillResult<Vec<DbRow>>> {
///     async move {
///     // bindings is the wit-bindgen-generated module in your component crate.
///     # mod bindings { pub mod flint { pub mod host { pub mod db {
///     #     pub struct HostError { pub code: String, pub message: String }
///     #     pub async fn query(_: &str, _: Vec<String>) -> Result<Vec<String>, HostError> { unimplemented!() }
///     # }}}}
///     let rows = bindings::flint::host::db::query(sql, params.to_vec())
///         .await
///         .map_err(|e| SkillError::from_host_error(
///             HostInterface::Db, e.code, e.message,
///         ))?;
///     rows.iter().map(|s| DbRow::from_json_str(s)).collect()
///     }
/// }
/// # }
/// ```
//
// Methods return `impl Future<Output = …> + Send` rather than `async fn` so
// that implementors (and the host) can rely on `Send` futures without a
// future breaking change. See the clippy::async_fn_in_trait lint.
pub trait Database {
    /// Execute `sql` with JSON-encoded `params`, returning one [`DbRow`] per
    /// result row.
    ///
    /// `params` is a list of JSON-encoded parameter values, one per `$N`
    /// placeholder in the SQL. The host decodes them at the boundary.
    fn query<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [String],
    ) -> impl Future<Output = SkillResult<Vec<DbRow>>> + Send;
}
