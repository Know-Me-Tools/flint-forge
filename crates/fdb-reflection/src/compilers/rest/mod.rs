use axum::{
    extract::{Query, State},
    http::HeaderMap,
    routing::{delete, get, patch, post},
    Extension, Json, Router,
};
use forge_identity::RlsContext;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;

use fdb_ports::{KetoCheck, SqlExecutor};
use forge_policy::Pep;

mod list;
mod mutations;
mod responses;
mod rpc;

use crate::model::DatabaseModel;
use crate::passes::endpoint_generation::{generate, EndpointKind};

// Re-exported so `mutations.rs` and `rpc.rs` can keep referring to these via
// `super::…` — the prior single-file surface is preserved even though the
// implementations now live in sibling modules.
use list::handle_list;
use responses::{
    bad_request, forbidden, insert_response, internal_error, json_bind, parse_filters,
    rows_response, KETO_NAMESPACE,
};

/// Shared state threaded into every route handler.
///
/// `executor` runs every CRUD/`rpc` SQL statement inside an RLS-scoped
/// transaction (`SqlExecutor::execute_raw` → `PgBackend::acquire(rls)`) — no
/// handler ever touches a raw, unscoped connection. `keto` and `pep` are
/// optional gates. When present, mutation handlers call them before executing
/// SQL and return `403` on denial; when absent (early boot, tests) the gates
/// are skipped. All fields are `Send + Sync`, satisfying the web-domain
/// requirement that handler state be shareable across worker threads.
#[derive(Clone)]
pub(super) struct RestState {
    pub(super) model: Arc<DatabaseModel>,
    pub(super) executor: Arc<dyn SqlExecutor>,
    pub(super) keto: Option<Arc<dyn KetoCheck>>,
    pub(super) pep: Option<Arc<dyn Pep>>,
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
/// Every handler runs its SQL through `executor.execute_raw(sql, params, rls)`
/// (`SqlExecutor`, `fdb-ports`), which `acquire(rls)`s a connection and issues
/// the `SET LOCAL` GUCs before the statement runs — RLS is enforced on every
/// route this compiler mounts, not assumed from the pool's ambient role.
/// `handle_rpc` additionally detects `vector(N)` arg types and binds
/// `pgvector::Vector` typed parameters automatically.
pub struct RestCompiler;

impl RestCompiler {
    /// Compile without authorization gates (early boot / tests). Mutations run
    /// with RLS only. Prefer [`RestCompiler::compile_with_gates`] in production.
    pub fn compile(model: &DatabaseModel, executor: Arc<dyn SqlExecutor>) -> Router {
        Self::compile_with_gates(model, executor, None, None)
    }

    /// Compile with optional Keto (coarse relationship) and Cedar (capability)
    /// gates. When supplied, every mutation handler calls both before touching
    /// SQL and returns `403` on denial.
    pub fn compile_with_gates(
        model: &DatabaseModel,
        executor: Arc<dyn SqlExecutor>,
        keto: Option<Arc<dyn KetoCheck>>,
        pep: Option<Arc<dyn Pep>>,
    ) -> Router {
        let state = RestState {
            model: Arc::new(model.clone()),
            executor,
            keto,
            pep,
        };

        let endpoints = generate(model);

        let mut router: Router<RestState> = Router::new();

        // Every route below is a concrete, literal path — `schema`/`table` (or
        // `func`'s schema/name) are compile-time-known from the `DatabaseModel`,
        // not runtime path parameters (axum never captures them: there is no
        // `{schema}`/`{table}` segment in any registered path). Each handler
        // closure binds its own table/function identity via capture instead of
        // a `Path` extractor — the extractor approach silently mismatched (0
        // captured segments vs. 2 expected) and every CRUD/`rpc` request 500'd
        // before reaching a handler body. Discovered running this change's own
        // live-Postgres gate test.
        for endpoint in &endpoints {
            let path = endpoint.path.clone();
            router = match (&endpoint.kind, endpoint.method) {
                (EndpointKind::TableList { table }, "GET") => {
                    let schema = table.schema.clone();
                    let name = table.name.clone();
                    router.route(
                        &path,
                        get(move |State(state): State<RestState>,
                                  Extension(rls): Extension<RlsContext>,
                                  Query(params): Query<HashMap<String, String>>,
                                  headers: HeaderMap| {
                            let schema = schema.clone();
                            let name = name.clone();
                            async move {
                                handle_list(state, rls, schema, name, params, headers).await
                            }
                        }),
                    )
                }
                (EndpointKind::TableList { table }, "POST") => {
                    let schema = table.schema.clone();
                    let name = table.name.clone();
                    router.route(
                        &path,
                        post(move |State(state): State<RestState>,
                                   Extension(rls): Extension<RlsContext>,
                                   Json(body): Json<Map<String, Value>>| {
                            let schema = schema.clone();
                            let name = name.clone();
                            async move {
                                mutations::handle_insert(state, schema, name, rls, Json(body)).await
                            }
                        }),
                    )
                }
                (EndpointKind::TableById { table }, "PATCH") => {
                    let schema = table.schema.clone();
                    let name = table.name.clone();
                    router.route(
                        &path,
                        patch(move |State(state): State<RestState>,
                                    Extension(rls): Extension<RlsContext>,
                                    Query(params): Query<HashMap<String, String>>,
                                    Json(body): Json<Map<String, Value>>| {
                            let schema = schema.clone();
                            let name = name.clone();
                            async move {
                                mutations::handle_update(
                                    state,
                                    schema,
                                    name,
                                    rls,
                                    Query(params),
                                    Json(body),
                                )
                                .await
                            }
                        }),
                    )
                }
                (EndpointKind::TableById { table }, "DELETE") => {
                    let schema = table.schema.clone();
                    let name = table.name.clone();
                    router.route(
                        &path,
                        delete(move |State(state): State<RestState>,
                                     Extension(rls): Extension<RlsContext>,
                                     Query(params): Query<HashMap<String, String>>| {
                            let schema = schema.clone();
                            let name = name.clone();
                            async move {
                                mutations::handle_delete(state, schema, name, rls, Query(params))
                                    .await
                            }
                        }),
                    )
                }
                (EndpointKind::Rpc { func }, "POST") => {
                    let schema = func.schema.clone();
                    let name = func.name.clone();
                    router.route(
                        &path,
                        post(move |State(state): State<RestState>,
                                   Extension(rls): Extension<RlsContext>,
                                   Json(body): Json<Map<String, Value>>| {
                            let schema = schema.clone();
                            let name = name.clone();
                            async move {
                                rpc::handle_rpc(state, rls, schema, name, Json(body)).await
                            }
                        }),
                    )
                }
                _ => router,
            };
        }

        router.with_state(state)
    }
}

#[cfg(test)]
mod tests {
    use super::RestCompiler;
    use crate::model::{DatabaseModel, Table};
    use fdb_query::QueryParam;
    use serde_json::Value;

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

    /// Never-invoked `SqlExecutor` for tests that only exercise route
    /// registration (`RestCompiler::compile`), which never runs a query.
    struct UnusedExecutor;
    #[async_trait::async_trait]
    impl fdb_ports::SqlExecutor for UnusedExecutor {
        async fn execute_raw(
            &self,
            _sql: &str,
            _params: Vec<QueryParam>,
            _rls: &forge_identity::RlsContext,
        ) -> Result<Vec<serde_json::Map<String, Value>>, fdb_ports::BackendError> {
            unreachable!("compile() never executes a query")
        }
    }

    #[tokio::test]
    async fn compiles_without_panic_for_minimal_model() {
        // compile() must not panic during route registration; it never runs a query.
        let model = minimal_model();
        let _router = RestCompiler::compile(&model, std::sync::Arc::new(UnusedExecutor));
    }

    // handle_list / build_inner_query / parse_range tests live in `list.rs`.
    // mutation_guard tests live in `mutations.rs`.
    // json_to_vector tests live in `rpc.rs`.
}
