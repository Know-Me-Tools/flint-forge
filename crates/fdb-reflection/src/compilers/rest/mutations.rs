//! REST mutation handlers: insert / update / delete, each gated by Keto + Cedar.
//!
//! Split out of `rest/mod.rs` to keep both files under the 500-line limit.
//! Shared state (`RestState`) and response/SQL helpers live in the parent
//! module and are re-used here via `super::`.

use axum::{http::StatusCode, response::IntoResponse};
use forge_domain::is_safe_identifier;
use forge_identity::RlsContext;
use forge_policy::{Decision, Request as PolicyRequest};
use serde_json::{Map, Value};
use std::collections::HashMap;
use tracing::instrument;

use crate::compilers::filters::{
    bind_mutation_value, bind_param, cast_hints_for, mutation_placeholder, mutation_value_to_bind,
    render_where_with_hints,
};

use super::{
    bad_request, forbidden, insert_response, internal_error, parse_filters, rows_response,
    RestState, KETO_NAMESPACE,
};

/// Run the mutation authorization gates for `<schema>.<table>` under `rls`.
///
/// Order (per §2.3): Keto coarse relationship check first, then Cedar
/// capability policy. Returns `Ok(())` when both allow (or when a gate is not
/// configured); returns a boxed `403` response on the first denial. `action` is
/// one of `insert` / `update` / `delete`.
///
/// SECURITY: the subject is PII — it is passed to Keto but never logged, and
/// the `403` body carries no subject or claim detail.
pub(super) async fn mutation_guard(
    state: &RestState,
    rls: &RlsContext,
    schema: &str,
    table: &str,
    action: &str,
) -> Result<(), Box<axum::response::Response>> {
    let object = format!("{schema}.{table}");

    // Keto coarse relationship check (fail-closed inside the adapter).
    if let Some(keto) = &state.keto {
        let allowed = keto
            .check(KETO_NAMESPACE, &object, action, &rls.keto_subject)
            .await;
        if !allowed {
            return Err(Box::new(forbidden()));
        }
    }

    // Cedar capability policy.
    if let Some(pep) = &state.pep {
        let req = PolicyRequest {
            action: action.to_owned(),
            resource: object,
            context: Value::Null,
        };
        if pep.check(rls, &req).await == Decision::Deny {
            return Err(Box::new(forbidden()));
        }
    }

    Ok(())
}

/// `POST /<schema>/<table>` — insert a row under RLS, gated by Keto + Cedar.
///
/// Body is a JSON object of `column: value`. Every column passes
/// [`is_safe_identifier`]; all values are bound as `$n`. Returns `201 Created`
/// with the inserted row and a `Location: /<schema>/<table>/<pk>` header.
#[instrument(skip(state, rls, body), fields(schema = %schema, table = %table))]
pub(super) async fn handle_insert(
    schema: String,
    table: String,
    state: RestState,
    rls: RlsContext,
    body: Map<String, Value>,
) -> impl IntoResponse {
    if !is_safe_identifier(&schema) || !is_safe_identifier(&table) {
        return bad_request("invalid schema or table identifier");
    }
    if let Err(resp) = mutation_guard(&state, &rls, &schema, &table, "insert").await {
        return *resp;
    }
    if body.is_empty() {
        return bad_request("insert body must not be empty");
    }

    // Validate every column name before it is interpolated.
    let hints = cast_hints_for(&state.model, &schema, &table);
    let mut columns: Vec<String> = Vec::with_capacity(body.len());
    let mut binds = Vec::with_capacity(body.len());
    let mut placeholders: Vec<String> = Vec::with_capacity(body.len());
    for (idx, (col, val)) in (1_usize..).zip(&body) {
        if !is_safe_identifier(col) {
            return bad_request(&format!("invalid column identifier: {col}"));
        }
        let bind = mutation_value_to_bind(val);
        placeholders.push(mutation_placeholder(idx, &bind, hints.get(col)));
        columns.push(col.clone());
        binds.push(bind);
    }

    let sql = format!(
        "INSERT INTO {schema}.{table} ({cols}) VALUES ({ph}) RETURNING row_to_json({table}) AS row",
        cols = columns.join(", "),
        ph = placeholders.join(", "),
    );

    let mut q = sqlx::query(&sql);
    for b in &binds {
        q = bind_mutation_value(q, b);
    }

    match q.fetch_one(&state.pool).await {
        Ok(row) => insert_response(&row, &schema, &table),
        Err(e) => {
            tracing::error!(error = %e, "handle_insert query error");
            internal_error()
        }
    }
}

/// `PATCH /<schema>/<table>` — update rows matching the query filter, gated.
///
/// Query params select rows (same operator dispatch as `handle_list`); the JSON
/// body supplies `SET` column/value pairs. Returns `200 OK` with the updated
/// rows, or `204 No Content` when nothing matched.
#[instrument(skip(state, rls, params, body), fields(schema = %schema, table = %table))]
pub(super) async fn handle_update(
    schema: String,
    table: String,
    state: RestState,
    rls: RlsContext,
    params: HashMap<String, String>,
    body: Map<String, Value>,
) -> impl IntoResponse {
    if !is_safe_identifier(&schema) || !is_safe_identifier(&table) {
        return bad_request("invalid schema or table identifier");
    }
    if let Err(resp) = mutation_guard(&state, &rls, &schema, &table, "update").await {
        return *resp;
    }
    if body.is_empty() {
        return bad_request("update body must not be empty");
    }

    // SET clause — validate columns, bind values starting at $1.
    let hints = cast_hints_for(&state.model, &schema, &table);
    let mut set_parts: Vec<String> = Vec::with_capacity(body.len());
    let mut binds = Vec::new();
    let mut idx = 1_usize;
    for (col, val) in &body {
        if !is_safe_identifier(col) {
            return bad_request(&format!("invalid column identifier: {col}"));
        }
        let bind = mutation_value_to_bind(val);
        set_parts.push(format!(
            "{col} = {}",
            mutation_placeholder(idx, &bind, hints.get(col))
        ));
        binds.push(bind);
        idx += 1;
    }

    let filter_tree = match parse_filters(&params) {
        Ok(f) => f,
        Err(resp) => return *resp,
    };
    let where_clause = match render_where_with_hints(&filter_tree, idx, &hints) {
        Ok(wc) => wc,
        Err(msg) => return bad_request(&msg),
    };

    let sql = format!(
        "UPDATE {schema}.{table} SET {set} {where_sql} RETURNING row_to_json({table}) AS row",
        set = set_parts.join(", "),
        where_sql = where_clause.sql,
    );

    let mut q = sqlx::query(&sql);
    for b in &binds {
        q = bind_mutation_value(q, b);
    }
    for b in &where_clause.binds {
        q = bind_param(q, b);
    }

    match q.fetch_all(&state.pool).await {
        Ok(rows) if rows.is_empty() => StatusCode::NO_CONTENT.into_response(),
        Ok(rows) => rows_response(&rows),
        Err(e) => {
            tracing::error!(error = %e, "handle_update query error");
            internal_error()
        }
    }
}

/// `DELETE /<schema>/<table>` — delete rows matching the query filter, gated.
///
/// Returns `204 No Content`.
#[instrument(skip(state, rls, params), fields(schema = %schema, table = %table))]
pub(super) async fn handle_delete(
    schema: String,
    table: String,
    state: RestState,
    rls: RlsContext,
    params: HashMap<String, String>,
) -> impl IntoResponse {
    if !is_safe_identifier(&schema) || !is_safe_identifier(&table) {
        return bad_request("invalid schema or table identifier");
    }
    if let Err(resp) = mutation_guard(&state, &rls, &schema, &table, "delete").await {
        return *resp;
    }

    let hints = cast_hints_for(&state.model, &schema, &table);
    let filter_tree = match parse_filters(&params) {
        Ok(f) => f,
        Err(resp) => return *resp,
    };
    let where_clause = match render_where_with_hints(&filter_tree, 1, &hints) {
        Ok(wc) => wc,
        Err(msg) => return bad_request(&msg),
    };

    let sql = format!(
        "DELETE FROM {schema}.{table} {where_sql}",
        where_sql = where_clause.sql,
    );

    let mut q = sqlx::query(&sql);
    for b in &where_clause.binds {
        q = bind_param(q, b);
    }

    match q.execute(&state.pool).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            tracing::error!(error = %e, "handle_delete query error");
            internal_error()
        }
    }
}

#[cfg(test)]
mod tests {
    //! Mutation gate tests — exercise `mutation_guard` directly with mock
    //! Keto/Cedar gates. The guard runs BEFORE any SQL, so a lazy (never-dialed)
    //! pool is sufficient: a denial returns before the pool is touched.

    use super::mutation_guard;
    use crate::compilers::rest::RestState;
    use crate::model::{DatabaseModel, Table};
    use async_trait::async_trait;
    use axum::http::StatusCode;
    use fdb_ports::KetoCheck;
    use forge_identity::RlsContext;
    use forge_policy::{Decision, Pep, Request as PolicyRequest};
    use std::sync::Arc;

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

    struct FixedKeto(bool);
    #[async_trait]
    impl KetoCheck for FixedKeto {
        async fn check(&self, _ns: &str, _obj: &str, _rel: &str, _subj: &str) -> bool {
            self.0
        }
    }

    struct FixedPep(Decision);
    #[async_trait]
    impl Pep for FixedPep {
        async fn check(&self, _who: &RlsContext, _req: &PolicyRequest) -> Decision {
            self.0
        }
    }

    fn test_rls() -> RlsContext {
        RlsContext {
            role: "authenticated".into(),
            claims_json: r#"{"sub":"user-1"}"#.into(),
            raw_bearer: "token".into(),
            keto_subject: "user-1".into(),
            vault_key_id: None,
        }
    }

    fn state_with_gates(keto: Option<Arc<dyn KetoCheck>>, pep: Option<Arc<dyn Pep>>) -> RestState {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test").expect("lazy pool");
        RestState {
            model: Arc::new(minimal_model()),
            pool,
            keto,
            pep,
        }
    }

    #[tokio::test]
    async fn guard_allows_when_no_gates_configured() {
        let state = state_with_gates(None, None);
        let r = mutation_guard(&state, &test_rls(), "public", "items", "insert").await;
        assert!(r.is_ok(), "no gates ⇒ allow");
    }

    #[tokio::test]
    async fn guard_denies_on_keto_reject() {
        let state = state_with_gates(Some(Arc::new(FixedKeto(false))), None);
        let r = mutation_guard(&state, &test_rls(), "public", "items", "update").await;
        assert!(r.is_err(), "keto false ⇒ 403");
        assert_eq!(r.unwrap_err().status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn guard_denies_on_cedar_deny_even_when_keto_allows() {
        let state = state_with_gates(
            Some(Arc::new(FixedKeto(true))),
            Some(Arc::new(FixedPep(Decision::Deny))),
        );
        let r = mutation_guard(&state, &test_rls(), "public", "items", "delete").await;
        assert!(r.is_err(), "cedar deny ⇒ 403");
        assert_eq!(r.unwrap_err().status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn guard_allows_when_both_gates_pass() {
        let state = state_with_gates(
            Some(Arc::new(FixedKeto(true))),
            Some(Arc::new(FixedPep(Decision::Allow))),
        );
        let r = mutation_guard(&state, &test_rls(), "public", "items", "insert").await;
        assert!(r.is_ok(), "keto allow + cedar allow ⇒ ok");
    }

    /// Injection-attempt: an unsafe column name is rejected by the fdb-query
    /// bridge at render time, so it can never reach a WHERE clause for a mutation.
    #[test]
    fn injection_column_rejected_before_sql() {
        use crate::compilers::filters::{parse_filter_tree, render_where};
        use forge_domain::is_safe_identifier;
        use std::collections::HashMap;
        let mut params = HashMap::new();
        params.insert("id; DROP TABLE users--".to_owned(), "eq.1".to_owned());
        let tree = parse_filter_tree(&params).expect("parses to a leaf");
        assert!(
            render_where(&tree, 1).is_err(),
            "unsafe column rejected at render"
        );
        assert!(is_safe_identifier("id"));
        assert!(!is_safe_identifier("id; DROP TABLE users--"));
    }
}
