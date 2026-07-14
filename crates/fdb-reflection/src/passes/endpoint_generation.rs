use crate::model::{DatabaseModel, FnMeta, Table};

/// Describes one HTTP endpoint derived from the `DatabaseModel`.
#[derive(Debug, Clone)]
pub struct Endpoint {
    /// HTTP method, e.g. `"GET"`, `"POST"`, `"PATCH"`, `"DELETE"`.
    pub method: &'static str,
    /// Literal route path `RestCompiler` registers with Axum, e.g.
    /// `/public/items` or `/rpc/public/find_similar`. Always a concrete,
    /// compile-time-known path — never a `{param}` template.
    pub path: String,
    /// What kind of handler this endpoint maps to, and the reflected item
    /// (table or function) that drives it.
    pub kind: EndpointKind,
}

/// The handler family an [`Endpoint`] maps to. `RestCompiler::compile_with_gates`
/// matches on `(kind, method)` to pick the concrete Axum handler.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum EndpointKind {
    /// Collection-level table access: `GET` (list) or `POST` (insert) on
    /// `/<schema>/<table>`.
    TableList {
        /// The table this endpoint operates on.
        table: Table,
    },
    /// Row-level table access: `PATCH` (update) or `DELETE` on the *same*
    /// collection path as [`Self::TableList`] — despite the name, rows are
    /// selected via PostgREST-style query filters (`?id=eq.5`), not a
    /// `{id}` path segment.
    TableById {
        /// The table this endpoint operates on.
        table: Table,
    },
    /// `POST /rpc/<schema>/<fn>` — call a reflected SQL function.
    Rpc {
        /// The function this endpoint invokes.
        func: FnMeta,
    },
}

/// Produce the ordered list of endpoints from a validated `DatabaseModel`.
/// Consumed by `RestCompiler::compile()`.
pub fn generate(model: &DatabaseModel) -> Vec<Endpoint> {
    let mut endpoints = Vec::new();

    for table in &model.tables {
        let prefix = format!("/{}/{}", table.schema, table.name);
        endpoints.push(Endpoint {
            method: "GET",
            path: prefix.clone(),
            kind: EndpointKind::TableList {
                table: table.clone(),
            },
        });
        endpoints.push(Endpoint {
            method: "POST",
            path: prefix.clone(),
            kind: EndpointKind::TableList {
                table: table.clone(),
            },
        });
        // PATCH/DELETE select rows via PostgREST-style query filters
        // (`?id=eq.5`), exactly like GET — not a path-parameterized `{id}`
        // segment, which `handle_update`/`handle_delete` never extract. Same
        // literal `prefix` as GET/POST; axum merges multiple `.route()` calls
        // for the same path into one `MethodRouter`.
        endpoints.push(Endpoint {
            method: "PATCH",
            path: prefix.clone(),
            kind: EndpointKind::TableById {
                table: table.clone(),
            },
        });
        endpoints.push(Endpoint {
            method: "DELETE",
            path: prefix.clone(),
            kind: EndpointKind::TableById {
                table: table.clone(),
            },
        });
    }

    for func in &model.functions {
        endpoints.push(Endpoint {
            method: "POST",
            path: format!("/rpc/{}/{}", func.schema, func.name),
            kind: EndpointKind::Rpc { func: func.clone() },
        });
    }

    endpoints
}
