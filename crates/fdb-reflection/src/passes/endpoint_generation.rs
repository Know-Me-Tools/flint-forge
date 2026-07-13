use crate::model::{DatabaseModel, FnMeta, Table};

/// Describes one HTTP endpoint derived from the `DatabaseModel`.
#[derive(Debug, Clone)]
pub struct Endpoint {
    pub method: &'static str,
    pub path: String,
    pub kind: EndpointKind,
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum EndpointKind {
    TableList { table: Table },
    TableById { table: Table },
    Rpc { func: FnMeta },
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
